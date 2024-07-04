use crate::global::opts::GLOBAL_OPTS;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    /// 数据库地址
    pub database_url: String,
    /// 服务器监听地址
    pub listen_addr: String,
    /// web监听地址
    pub web_addr: String,
    /// web目录
    pub web_base_dir: String,
}

pub static GLOBAL_CONFIG: Lazy<Config> = Lazy::new(|| {
    let file = match File::open(&GLOBAL_OPTS.config_file) {
        Ok(file) => file,
        Err(e) => {
            eprintln!("Failed to open config file: {}", e);
            std::process::exit(1);
        }
    };
    let reader = BufReader::new(file);
    match serde_json::from_reader(reader) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Failed to parse config file: {}", e);
            std::process::exit(1);
        }
    }
});
