use crate::config;
use crate::config::RunnerConfig;
use crate::http_api::{send_request, HttpRequest, HttpResponse};
use crate::scenario::Global;
use crate::scenario::Scenario;
use crate::script::ScriptContext;
use crate::stats::ApiStats;
use bytes::Bytes;
use h2::client;
use h2::client::SendRequest;
use std::cell::RefCell;
use std::error::Error;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::Instant;
use tokio::net::TcpStream;
use tokio::sync::mpsc::channel;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::time;
use tokio::time::Duration;

pub struct Runner {
    param: RunParameter,
    target_address: String,
    first_scenario: Scenario,
    subsequent_scenarios: Vec<Scenario>,
}

impl Runner {
    pub fn new(config: RunnerConfig) -> Result<Runner, Box<dyn Error>> {
        // batch size
        let batch_size = match config.batch_size {
            config::BatchSize::Auto(_) => None,
            config::BatchSize::Fixed(size) => Some(size),
        };

        // duration
        if config.duration.as_secs() <= 0 {
            return Err("Duration must be at least 1s".into());
        }
        let duration_s = config.duration.as_secs() as u32;

        // target address
        let url = config.base_url.clone();
        let address = url
            .strip_prefix("http://")
            .or_else(|| url.strip_prefix("https://"))
            .unwrap_or(&url);
        let address = address.trim_end_matches('/');

        // scenarios
        let first_scenario_config = config.requests.get(0).ok_or("No request defined")?;

        let mut subsequent_scenarios_config = vec![];
        for scenario_config in config.requests.iter().skip(1) {
            subsequent_scenarios_config.push(scenario_config);
        }
        let mut subsequent_scenarios = vec![];
        for scenario_config in subsequent_scenarios_config.iter() {
            subsequent_scenarios.push(Scenario::new(scenario_config, &config.base_url));
        }

        let scenario_count = subsequent_scenarios_config.len() + 1;

        Ok(Runner {
            param: RunParameter::new(config.target_rps, duration_s, batch_size, scenario_count),
            target_address: address.into(),
            first_scenario: Scenario::new(first_scenario_config, &config.base_url),
            subsequent_scenarios,
        })
    }

    pub async fn run(&mut self, global: Arc<RwLock<Global>>) -> Result<RunReport, Box<dyn Error>> {
        let tcp = TcpStream::connect(&self.target_address).await?;
        let (client, h2) = client::handshake(tcp).await?;

        tokio::task::spawn(async move {
            if let Err(e) = h2.await {
                log::error!("Unexpected error: {:?}", e);
            }
        });

        let (eventloop_tx, eventloop_rx) = channel(32);
        tokio::spawn(async move {
            // TODO remove unwrap
            Self::event_loop(client, eventloop_rx).await.unwrap();
        });

        let param = &self.param;
        let total_iterations = param.total_requests as f64 / param.batch_size as f64;
        let total_iterations = total_iterations.ceil() as u32;
        let scenario_count = self.param.scenario_count as u32;
        let total_requests = total_iterations * param.batch_size * scenario_count;

        log::info!(
            "Sending Total Req: {}, Iteration: {}, Target RPS: {} TPS: {}, Batch Size: {}, Interval: {}",
            total_requests,
            total_iterations,
            param.target_rps,
            param.target_tps,
            param.batch_size,
            param.interval.as_secs_f64()
        );

        let start = Instant::now();
        let api_stats = Arc::new(ApiStats::new());

        let mut interval = time::interval(param.interval);
        for _ in 0..total_iterations {
            interval.tick().await;

            let (resp_tx, mut resp_rx) = channel(32);

            for _ in 0..param.batch_size {
                let scenario = &mut self.first_scenario;
                log::debug!("Running scenario #0: {}", scenario.name);

                // First Pre Script
                let mut script_ctx = ScriptContext::new(Arc::clone(&global));
                scenario.run_pre_script(&mut script_ctx);

                // First HTTP request
                let http_request = scenario.new_request(&script_ctx).unwrap();

                let ctx = EventContext {
                    scenario_id: 0,
                    script_ctx: RefCell::new(script_ctx),
                };
                eventloop_tx
                    .send(Event::SendMessage(ctx, http_request, resp_tx.clone()))
                    .await?;
            }

            let total_response = param.batch_size * scenario_count;

            for _ in 0..total_response {
                if let Some((ctx, response)) = resp_rx.recv().await {
                    log::debug!("Response Status: {:?}", response.status);
                    log::debug!("Response Body: {:?}", response.body);
                    api_stats.inc_retry(response.retry_count.into());

                    // Get Scenario
                    let scenario_id = ctx.scenario_id;
                    let cur_scenario = if scenario_id == 0 {
                        &self.first_scenario
                    } else {
                        &self.subsequent_scenarios[scenario_id - 1]
                    };

                    // Success Stats
                    let round_trip_time = response.request_start.elapsed().as_micros() as u64;
                    api_stats.inc_rtt(round_trip_time);
                    api_stats.inc_success();

                    {
                        let mut script_ctx = ctx.script_ctx.borrow_mut();

                        // Get new variables from response to pass to next scenario
                        cur_scenario
                            .from_response(&mut script_ctx, &response)
                            .unwrap();

                        // Post scenario
                        cur_scenario.run_post_script(&mut script_ctx);
                    }

                    // TODO: Error Stats
                    // api_stats.inc_error();

                    // Check if there are subsequent scenarios
                    if let Some(scenario) = self.subsequent_scenarios.get_mut(scenario_id) {
                        log::debug!("Running scenario #{}: {}", scenario_id + 1, scenario.name);

                        // Pre Script
                        let http_request: HttpRequest;
                        {
                            let mut script_ctx = ctx.script_ctx.borrow_mut();
                            scenario.run_pre_script(&mut script_ctx);
                            http_request = scenario.new_request(&script_ctx).unwrap();
                        }

                        eventloop_tx
                            .send(Event::SendMessage(
                                EventContext {
                                    scenario_id: scenario_id + 1,
                                    script_ctx: ctx.script_ctx,
                                },
                                http_request,
                                resp_tx.clone(),
                            ))
                            .await?;
                    } else {
                        log::debug!("All scenarios completed");
                    }
                }
            }
        }

        // while api_stats.get_success() + api_stats.get_error() < total_requests {
        //     tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        // }

        // Terminate the event loop
        eventloop_tx.send(Event::Terminate).await.unwrap();

        let success_count = api_stats.get_success();
        let error_count = api_stats.get_error();
        let total_count = success_count + error_count;
        let total_rtt = Duration::from_micros(api_stats.get_rtt());
        let total_retry = api_stats.get_retry();

        let elapsed = start.elapsed();
        let elapsed_s = elapsed.as_secs() as f64 + elapsed.subsec_millis() as f64 / 1000.0;
        let rps = success_count as f64 / (elapsed.as_micros() as f64 / 1_000_000.0);
        let avg_rtt = total_rtt.as_millis() as f64 / success_count as f64;

        log::info!(
            "Elapsed: {:.3}s, RPS: {:.3}, RTT: {:.3}ms, Error: ({}/{}), Retry: {}",
            elapsed_s,
            rps,
            avg_rtt,
            error_count,
            total_count,
            total_retry
        );

        let report = RunReport {
            rps,
            elapsed,
            success_count,
            error_count,
            total_rtt,
        };
        Ok(report)
    }

    async fn event_loop(
        mut client: SendRequest<Bytes>,
        mut rx: Receiver<Event>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        while let Some(event) = rx.recv().await {
            match event {
                Event::SendMessage(ctx, request, tx) => {
                    let future = send_request(&mut client, request).await?;
                    // TODO handle timeout?

                    let scenario_id = ctx.scenario_id;
                    log::debug!("Request {} sent", scenario_id);

                    tokio::spawn(async move {
                        let response = future.await.unwrap(); // handle error?
                        let response = response.unwrap(); // handle error?

                        tx.send((
                            EventContext {
                                scenario_id,
                                script_ctx: ctx.script_ctx,
                            },
                            response,
                        ))
                        .await
                        .unwrap(); // handle error?
                    });
                }
                Event::Terminate => {
                    log::info!("Terminating event loop");
                    break;
                }
            }
        }
        Ok(())
    }
}

struct EventContext {
    scenario_id: usize,
    script_ctx: RefCell<ScriptContext>,
}

enum Event {
    SendMessage(
        EventContext,
        HttpRequest,
        Sender<(EventContext, HttpResponse)>,
    ),
    Terminate,
}

#[derive(Clone)]
pub struct RunParameter {
    pub target_rps: u32,
    pub target_tps: u32,
    pub batch_size: u32,
    pub interval: Duration,
    pub total_requests: u32,
    pub scenario_count: usize,
}

impl RunParameter {
    pub fn new(
        target_rps: u32,
        duration_s: u32,
        batch_size: Option<u32>,
        scenario_count: usize,
    ) -> RunParameter {
        let target_tps = target_rps / scenario_count as u32;
        let target_tps = if target_tps == 0 { 1 } else { target_tps };

        let batch_size = if let Some(batch_size) = batch_size {
            batch_size
        } else {
            let batch_size = (target_tps / 200) as u32;
            let batch_size = if batch_size == 0 { 1 } else { batch_size };
            batch_size
        };
        let batches_per_second = target_tps as f64 / batch_size as f64;
        let interval = Duration::from_secs_f64(1.0 / batches_per_second);
        let total_requests = target_tps * duration_s;

        RunParameter {
            target_rps,
            target_tps,
            batch_size,
            interval,
            total_requests,
            scenario_count,
        }
    }
}

pub struct RunReport {
    pub rps: f64,
    pub elapsed: Duration,
    pub success_count: u32,
    pub error_count: u32,
    pub total_rtt: Duration,
}

pub struct AggregatedReport {
    pub total_rps: f64,
    pub elapsed: Duration,
    pub total_success: u32,
    pub total_error: u32,
    pub total_rtt: Duration,
}

impl AggregatedReport {
    pub fn new() -> AggregatedReport {
        AggregatedReport {
            total_rps: 0.0,
            elapsed: Duration::from_secs(0),
            total_success: 0,
            total_error: 0,
            total_rtt: Duration::from_secs(0),
        }
    }

    pub fn add(&mut self, report: RunReport) {
        self.total_rps += report.rps;
        self.elapsed = self.elapsed.max(report.elapsed);
        self.total_success += report.success_count;
        self.total_error += report.error_count;
        self.total_rtt += report.total_rtt;
    }

    pub fn report(&self) {
        let elapsed_s =
            self.elapsed.as_secs() as f64 + self.elapsed.subsec_millis() as f64 / 1000.0;

        let avg_rtt = self.total_rtt.as_millis() as f64 / self.total_success as f64;

        log::info!("Total RPS: {:.3}", self.total_rps);
        log::info!("Average Round Trip: {:.4}ms", avg_rtt);
        log::info!("Elapsed: {:.3}s", elapsed_s);
        log::info!(
            "Success Rate: {:.2}%",
            self.total_success as f64 / (self.total_success + self.total_error) as f64 * 100.0
        );
    }
}
