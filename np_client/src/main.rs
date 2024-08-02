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
    #[arg(long, default_value_t = false, action = clap::ArgAction::Set)]
    pub backtrace: bool,

    /// Server address
    #[arg(short, long)]
    pub server: String,

    /// username
    #[arg(short, long)]
    pub username: String,

    /// password
    #[arg(short, long)]
    pub password: String,

    /// Set log level  warn
    #[arg(long, default_value = "info")]
    pub log_level: String,

    /// Set log level
    #[arg(long, default_value = "error")]
    pub base_log_level: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let ops = Opts::parse();

    if ops.backtrace {
        env::set_var("RUST_BACKTRACE", "1")
    }

    // 日志初始化
    let _logger = Logger::try_with_str(format!(
        "{}, mio=error, np_base={}",
        ops.log_level, ops.base_log_level
    ))?
    .log_to_file(
        FileSpec::default()
            .directory("logs")
            .suppress_timestamp()
            .suffix("log"),
    )
    .duplicate_to_stdout(Duplicate::All)
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
