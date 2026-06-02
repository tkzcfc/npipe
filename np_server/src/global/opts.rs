use clap::Parser;
use once_cell::sync::Lazy;

/// 优先使用 CI 中设置的 BIN_VERSION（git tag），否则回退到 Cargo.toml 版本
const VERSION: &str = match option_env!("BIN_VERSION") {
    Some(v) => v,
    None => env!("CARGO_PKG_VERSION"),
};

pub static GLOBAL_OPTS: Lazy<Opts> = Lazy::new(Opts::parse);

#[derive(Parser)]
#[command(author = "https://github.com/tkzcfc/npipe", version = VERSION, about, long_about = None)]
pub struct Opts {
    /// Print backtracking information
    #[arg(short, long, default_value_t = false, action = clap::ArgAction::Set)]
    pub backtrace: bool,

    /// Config file
    #[arg(short, long, default_value = "config.json")]
    pub config_file: String,

    /// Set log level  warn
    #[arg(long, default_value = "info")]
    pub log_level: String,

    /// Set log level
    #[arg(long, default_value = "error")]
    pub base_log_level: String,
}
