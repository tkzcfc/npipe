mod callback_test;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Optional name to operate on
    name: Option<String>,

    /// Sets a custom config file
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    #[arg(short, long, default_value_t = false, action = clap::ArgAction::Set)]
    debug: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// does testing things
    Test {
        /// lists test values
        #[arg(short, long)]
        list: i32,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // You can check the value provided by positional arguments, or option arguments
    if let Some(name) = cli.name.as_deref() {
        println!("Value for name: {name}");
    }

    if let Some(config_path) = cli.config.as_deref() {
        println!("Value for config: {}", config_path.display());
    }

    // You can see how many times a particular flag or argument occurred
    // Note, only flags can have multiple occurrences
    println!("cli.debug: {}", cli.debug);
    // match cli.debug {
    //     0 => println!("Debug mode is off"),
    //     1 => println!("Debug mode is kind of on"),
    //     2 => println!("Debug mode is on"),
    //     _ => println!("Don't be crazy"),
    // }

    // You can check for the existence of subcommands, and if found use their
    // matches just as you would the top level cmd
    match &cli.command {
        Some(Commands::Test { list }) => {
            if *list == 0 {
                println!("Printing testing lists...");
            } else {
                println!("Not printing testing lists...");
            }
        }
        None => {}
    }

    callback_test::run().await;
    let mtx = Mutex::new(0);

    tokio::join!(work(&mtx), work(&mtx));

    println!("{}", *mtx.lock().await);
}

async fn work(mtx: &Mutex<i32>) {
    println!("lock");
    {
        let mut v = mtx.lock().await;
        println!("locked");
        // slow redis network request
        sleep(Duration::from_millis(100)).await;
        *v += 1;
    }
    println!("unlock")
}
