mod config;
mod http_api;
mod runner;
mod stats;

use crate::config::read_yaml_file;
use crate::runner::AggregatedReport;
use crate::runner::Runner;
use chrono::Local;
use std::error::Error;
use std::io::Write;
use std::thread;
use tokio::sync::mpsc;

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::Builder::new()
        .format(|buf, record| {
            let now = Local::now();
            let thread = thread::current();
            let thread_name = thread.name().unwrap_or("unnamed");
            let thread_id = thread.id();

            writeln!(
                buf,
                "{} [{}] {:?} - ({}): {}",
                now.format("%Y-%m-%d %H:%M:%S%.3f"),
                record.level(),
                thread_id,
                thread_name,
                record.args()
            )
        })
        .filter(None, log::LevelFilter::Info)
        .init();

    let config = read_yaml_file("config.yaml")?;
    log::info!("Config: {:?}", config);

    let (tx, mut rx) = mpsc::channel(8);

    // let target_tps = 1;
    // let duration_s = 2;
    // let parallel = 1;
    // let batch_size = None; // If None, it will be calculated based on target_tps automatically
    //                        // let batch_size = Some(2);
    let batch_size = match config.batch_size {
        config::BatchSize::Auto(_) => None,
        config::BatchSize::Fixed(size) => Some(size),
    };

    for _ in 0..config.parallel {
        let tx = tx.clone();
        tokio::task::spawn_blocking(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            rt.block_on(async move {
                let runner = Runner::new(config.target_tps, config.duration, batch_size);
                let report = runner.run().await.unwrap();
                tx.send(report).await.unwrap();
            });
        });
    }

    drop(tx);

    let mut aggregate_report = AggregatedReport::new();
    while let Some(report) = rx.recv().await {
        aggregate_report.add(report);
    }
    aggregate_report.report();

    Ok(())
}
