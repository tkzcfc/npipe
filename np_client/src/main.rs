use clap::Parser;
use flexi_logger::{Age, Cleanup, Criterion, Duplicate, FileSpec, Logger, Naming, WriteMode};
use log::error;
use std::env;
use std::time::Duration;
use tokio::time::sleep;

mod client;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Opts {
    /// Print backtracking information
    #[arg(short, long, default_value_t = false, action = clap::ArgAction::Set)]
    pub backtrace: bool,

    /// Server address
    #[arg(long, default_value = "127.0.0.1:8118")]
    pub server_addr: String,

    /// username
    #[arg(short, long, default_value = "f123456")]
    pub username: String,

    /// password
    #[arg(short, long, default_value = "f123456")]
    pub password: String,

    /// Set log level  warn
    #[arg(long, default_value = "trace")]
    pub log_level: String,
}

fn duplicate_level(val: &str) -> Duplicate {
    match val {
        "none" => Duplicate::None,
        "error" => Duplicate::Error,
        "warn" => Duplicate::Warn,
        "info" => Duplicate::Info,
        "debug" => Duplicate::Debug,
        "trace" => Duplicate::Trace,
        "all" => Duplicate::All,
        _ => Duplicate::All,
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let ops = Opts::parse();

    if ops.backtrace {
        env::set_var("RUST_BACKTRACE", "1")
    }

    // 日志初始化
    let _logger = Logger::try_with_str("trace")?
        .log_to_file(
            FileSpec::default()
                .directory("logs")
                .suppress_timestamp()
                .suffix("log"),
        )
        .duplicate_to_stdout(duplicate_level(ops.log_level.as_str()))
        .format(flexi_logger::opt_format)
        .format_for_stdout(flexi_logger::colored_opt_format)
        .rotate(
            Criterion::AgeOrSize(Age::Day, 1024 * 1024 * 5),
            Naming::Numbers,
            Cleanup::KeepLogFiles(30),
        )
        .print_message()
        .write_mode(WriteMode::Async)
        .start()?;

    loop {
        if let Err(err) = client::run(&ops).await {
            error!("{err}");
            sleep(Duration::from_secs(5)).await;
        } else {
            sleep(Duration::from_millis(100)).await;
        }
    }
}
