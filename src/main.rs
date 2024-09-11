mod config;
mod error;
mod function;
mod http_api;
mod runner;
mod scenario;
mod script;
mod scripting;
mod stats;
mod variable;

use crate::config::read_yaml_file;
use crate::runner::AggregatedReport;
use crate::runner::Runner;
use crate::scenario::Global;
use crate::script::ScriptContext;
use crate::scripting::Scripts;
use chrono::Local;
use clap::Parser;
use std::error::Error;
use std::io::Write;
use std::sync::Arc;
use std::sync::RwLock;
use std::thread;
use tokio::sync::mpsc;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "./config.yaml")]
    config: String,

    #[arg(short, long, default_value_t = 1)]
    count: u8,

    #[arg(short, long)]
    overrides: Vec<String>,
}

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    // Read config
    let config = read_yaml_file(&args.config, args.overrides)?;

    // Configure Logging
    env_logger::Builder::new()
        .filter_module("jsonpath_lib", log::LevelFilter::Error)
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
        .filter(None, config.log_level.into())
        .init();

    log::debug!("HTTP2 Load Generator Config:\n{}", config);

    // Runner in parallel
    let (tx, mut rx) = mpsc::channel(8);
    for _ in 0..config.parallel {
        let tx = tx.clone();
        let config = config.clone();
        tokio::task::spawn_blocking(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            rt.block_on(async move {
                let global = Global::new(config.runner.global.clone());
                let global = Arc::new(RwLock::new(global));
                {
                    let init_scripts = Scripts::parse(&config.runner.global.scripts).unwrap();
                    let mut ctx = ScriptContext::new(Arc::clone(&global));
                    init_scripts.execute(&mut ctx).unwrap();
                    ctx.save_variables_as_global();
                }

                let mut runner = Runner::new(config.runner).unwrap();
                let report = runner.run(global).await.unwrap();
                tx.send(report).await.unwrap();
            });
        });
    }

    drop(tx);

    // Aggregate report
    let mut aggregate_report = AggregatedReport::new();
    while let Some(report) = rx.recv().await {
        aggregate_report.add(report);
    }
    aggregate_report.report();

    Ok(())
}
