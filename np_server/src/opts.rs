
use clap::{Parser};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Opts {
    /// Print backtracking information
    #[arg(short, long, default_value_t = false, action = clap::ArgAction::Set)]
    backtrace: bool,
}
