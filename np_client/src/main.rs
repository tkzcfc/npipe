use clap::{Args, Parser, Subcommand};
use flexi_logger::{
    Age, Cleanup, Criterion, Duplicate, FileSpec, Logger, LoggerHandle, Naming, WriteMode,
};
use http::Uri;
use log::{error, info};
use once_cell::sync::OnceCell;
use std::str::FromStr;
use std::time::Duration;
use std::{env, panic};
use tokio::time::sleep;

mod client;
#[cfg(windows)]
mod winservice;

/// Common runtime arguments shared by process mode and Windows service mode.
///
/// Transport options live here so interactive startup and service startup use the same pool behavior.
#[derive(Args)]
struct CommonArgs {
    /// Print backtracking information.
    #[arg(long, default_value_t = false, action = clap::ArgAction::Set)]
    pub backtrace: bool,

    /// Server address.
    #[arg(short, long)]
    pub server: String,

    /// Username.
    #[arg(short, long)]
    pub username: String,

    /// Password.
    #[arg(short, long)]
    pub password: String,

    /// Enable TLS.
    #[arg(long, default_value = "false")]
    pub enable_tls: bool,

    /// TLS server name. If empty, the host from the server address is used.
    #[arg(long, default_value = "")]
    pub tls_server_name: String,

    /// Skip server certificate verification.
    #[arg(long, default_value = "false")]
    pub insecure: bool,

    /// Quiet mode. Do not print logs.
    #[arg(long, default_value = "false")]
    pub quiet: bool,

    /// Custom CA certificate path. If empty, system roots are used.
    #[arg(long, default_value = "")]
    pub ca_cert: String,

    /// Maximum number of forward connections/streams. 0 keeps legacy single-connection mode.
    #[arg(long, default_value_t = 16)]
    pub transport_max_connections: u32,

    /// Minimum number of forward connections to keep alive (pre-warmed). 0 disables warm-up.
    #[arg(long, default_value_t = 4)]
    pub transport_min_connections: u32,

    /// Idle timeout for forward connections, in seconds.
    #[arg(long, default_value_t = 60)]
    pub transport_idle_timeout_secs: u32,

    /// Client log level.
    #[arg(long, default_value = "info")]
    pub log_level: String,

    /// Base library log level.
    #[arg(long, default_value = "error")]
    pub base_log_level: String,

    /// Log directory.
    #[arg(long, default_value = "logs")]
    pub log_dir: String,
}

/// 优先使用 CI 中设置的 BIN_VERSION（git tag），否则回退到 Cargo.toml 版本
const VERSION: &str = match option_env!("BIN_VERSION") {
    Some(v) => v,
    None => env!("CARGO_PKG_VERSION"),
};

#[derive(Parser)]
#[command(author = "https://github.com/tkzcfc/npipe", version = VERSION, about, long_about = None)]
struct Opts {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[cfg(not(windows))]
#[derive(Subcommand)]
enum Commands {
    /// Run.
    Run {
        #[command(flatten)]
        common_args: CommonArgs,
    },
}

#[cfg(windows)]
#[derive(Subcommand)]
enum Commands {
    /// Installs service.
    Install {
        #[command(flatten)]
        common_args: CommonArgs,
    },
    /// Uninstalls service.
    Uninstall,

    /// Run as Windows service.
    RunService {
        #[command(flatten)]
        common_args: CommonArgs,
    },

    /// Run.
    Run {
        #[command(flatten)]
        common_args: CommonArgs,
    },
}

// 全局日志记录器
static LOGGER: OnceCell<LoggerHandle> = OnceCell::new();

fn init_logger(common_args: &CommonArgs) -> anyhow::Result<()> {
    if common_args.backtrace {
        unsafe { env::set_var("RUST_BACKTRACE", "1") }
    }

    if common_args.quiet {
        return Ok(());
    }

    // 日志初始化
    let logger = Logger::try_with_str(format!(
        "{}, mio=error, np_base={}",
        common_args.log_level, common_args.base_log_level
    ))?
    .log_to_file(
        FileSpec::default()
            .directory(&common_args.log_dir)
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

    // panic::set_hook(Box::new(|panic_info| {
    //     error!("Panic occurred: {:?}", panic_info);
    // }));

    Ok(())
}

async fn run_with_args(common_args: CommonArgs) -> anyhow::Result<()> {
    let mut uri_cycle_iter = common_args
        .server
        .split(",")
        .filter_map(|s| {
            Uri::from_str(s)
                .map_err(|e| {
                    error!("Failed to parse URI '{}': {}", s, e);
                    e
                })
                .ok() // 丢弃错误，保留成功的 Uri
        })
        .collect::<Vec<_>>()
        .into_iter() // 转换为 owned iterator 避免生命周期问题
        .cycle();

    loop {
        if let Some(uri) = uri_cycle_iter.next() {
            info!("Starting client with server URI: {}", uri);
            if let Err(err) = client::run(&common_args, uri).await {
                error!("Client run error: {}", err);
                sleep(Duration::from_secs(5)).await;
            } else {
                info!("Client exited normally, restarting...");
                sleep(Duration::from_secs(1)).await;
            }
        } else {
            error!("No valid uri found");
            return Err(anyhow::anyhow!("No valid uri found"));
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
            run_with_args(common_args).await
        }
        _ => {
            panic!("unknown command")
        }
    }
}
