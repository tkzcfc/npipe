use crate::global::opts::GLOBAL_OPTS;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    /// 数据库地址
    pub database_url: String,
    /// 服务监听地址
    #[serde(default = "default_config_empty_string_function")]
    pub listen_addr: String,
    /// 启用tls
    pub enable_tls: bool,
    /// tls证书
    pub tls_cert: String,
    /// tls秘钥
    pub tls_key: String,
    /// web监听地址
    pub web_addr: String,
    /// 管理员用户
    pub web_username: String,
    /// 管理员密码
    pub web_password: String,
    /// web目录
    pub web_base_dir: String,
    /// 非法流量转发地址
    #[serde(default = "default_config_empty_string_function")]
    pub illegal_traffic_forward: String,
    /// 安静模式下不输出日志
    #[serde(default = "default_config_quiet_function")]
    pub quiet: bool,
    /// 日志保存路径
    #[serde(default = "default_config_log_dir_function")]
    pub log_dir: String,
}

fn default_config_empty_string_function() -> String {
    "".to_string()
}
fn default_config_quiet_function() -> bool {
    false
}
fn default_config_log_dir_function() -> String {
    "logs".to_string()
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
