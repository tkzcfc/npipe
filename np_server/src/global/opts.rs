use clap::Parser;
use once_cell::sync::Lazy;

pub static GLOBAL_OPTS: Lazy<Opts> = Lazy::new(|| Opts::parse());

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Opts {
    /// Print backtracking information
    #[arg(short, long, default_value_t = false, action = clap::ArgAction::Set)]
    pub backtrace: bool,

    /// Set log level
    #[arg(long, default_value = "warn")]
    pub log_level: String,
}
