use clap::{Args, Parser, Subcommand};
use flexi_logger::{
    Age, Cleanup, Criterion, Duplicate, FileSpec, Logger, LoggerHandle, Naming, WriteMode,
};
use log::error;
use once_cell::sync::OnceCell;
use std::time::Duration;
use std::{env, panic};
use tokio::time::sleep;

mod client;
#[cfg(windows)]
mod winservice;

#[derive(Args)]
pub(crate) struct CommonArgs {
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

    /// enable tls
    #[arg(long, default_value = "false")]
    pub enable_tls: bool,

    /// ca certificate
    #[arg(long, default_value = "")]
    pub ca_cert: String,

    /// Set log level  warn
    #[arg(long, default_value = "info")]
    pub log_level: String,

    /// Set log level
    #[arg(long, default_value = "error")]
    pub base_log_level: String,
}

#[derive(Parser)]
#[command(author = "https://github.com/tkzcfc/npipe", version, about, long_about = None)]
pub struct Opts {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[cfg(not(windows))]
#[derive(Subcommand)]
enum Commands {
    /// Run
    Run {
        #[command(flatten)]
        common_args: CommonArgs,
    },
}

#[cfg(windows)]
#[derive(Subcommand)]
enum Commands {
    /// Installs service
    Install {
        #[command(flatten)]
        common_args: CommonArgs,
    },
    /// Uninstalls service
    Uninstall,

    /// Run as windows service
    RunService {
        #[command(flatten)]
        common_args: CommonArgs,
    },

    /// Run
    Run {
        #[command(flatten)]
        common_args: CommonArgs,
    },
}

// 全局日志记录器
static LOGGER: OnceCell<LoggerHandle> = OnceCell::new();

pub(crate) fn init_logger(common_args: &CommonArgs) -> anyhow::Result<()> {
    if common_args.backtrace {
        unsafe { env::set_var("RUST_BACKTRACE", "1") }
    }

    // 日志初始化
    let logger = Logger::try_with_str(format!(
        "{}, mio=error, np_base={}",
        common_args.log_level, common_args.base_log_level
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
    .write_mode(WriteMode::Async);

    if LOGGER.set(logger.start()?).is_err() {
        panic!("set logger error");
    }

    panic::set_hook(Box::new(|panic_info| {
        error!("Panic occurred: {:?}", panic_info);
    }));

    Ok(())
}

pub(crate) async fn run_with_args(common_args: CommonArgs) -> anyhow::Result<()> {
    loop {
        if let Err(err) = client::run(&common_args).await {
            error!("{err}");
            sleep(Duration::from_secs(5)).await;
        } else {
            sleep(Duration::from_millis(100)).await;
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let ops = Opts::parse();

    #[cfg(windows)]
    {
        match ops.command {
            Some(Commands::Install { common_args }) => {
                return winservice::install_service(common_args);
            }
            Some(Commands::Uninstall) => {
                return winservice::uninstall_service();
            }
            Some(Commands::RunService { common_args }) => {
                return winservice::run_server_as_service(common_args);
            }
            _ => {}
        }
    }

    match ops.command {
        Some(Commands::Run { common_args }) => {
            init_logger(&common_args)?;
            return run_with_args(common_args).await;
        }
        _ => {
            panic!("unknown command")
        }
    }
}
