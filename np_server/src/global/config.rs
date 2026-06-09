use crate::global::forward_rule::ForwardRule;
use crate::global::opts::GLOBAL_OPTS;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;

#[derive(Serialize, Deserialize, Debug)]
pub struct ForwardRuleConfig {
    #[serde(default = "default_config_empty_string_function")]
    pub name: String,
    pub match_expr: String,
    pub target: String,
}

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
    /// Web 管理后台是否启用 TLS
    #[serde(default = "default_config_false_function")]
    pub web_enable_tls: bool,
    /// Web 管理后台 TLS 证书
    #[serde(default = "default_config_empty_string_function")]
    pub web_tls_cert: String,
    /// Web 管理后台 TLS 私钥
    #[serde(default = "default_config_empty_string_function")]
    pub web_tls_key: String,
    /// Web 管理后台证书未配置时是否自动生成临时自签名证书
    #[serde(default = "default_config_false_function")]
    pub web_tls_auto_self_signed: bool,
    /// Web 管理后台 Cookie 是否强制 Secure
    #[serde(default = "default_config_false_function")]
    pub web_cookie_secure: bool,
    /// 管理员用户
    pub web_username: String,
    /// 管理员密码
    pub web_password: String,
    /// web目录
    pub web_base_dir: String,
    /// 非法流量转发地址
    #[serde(default = "default_config_empty_string_function")]
    pub illegal_traffic_forward: String,
    /// 非法流量转发规则
    #[serde(default)]
    pub illegal_traffic_forward_rules: Vec<ForwardRuleConfig>,
    /// 安静模式下不输出日志
    #[serde(default = "default_config_quiet_function")]
    pub quiet: bool,
    /// 日志保存路径
    #[serde(default = "default_config_log_dir_function")]
    pub log_dir: String,
    /// 每个用户允许的最大转发连接数，0 表示只使用单连接模式
    #[serde(default = "default_config_transport_max_connections_function")]
    pub transport_max_connections_per_player: u32,
    /// 转发连接空闲关闭时间（秒）
    #[serde(default = "default_config_transport_idle_timeout_secs_function")]
    pub transport_idle_timeout_secs: u32,
    #[serde(skip)]
    pub forward_rules: Vec<ForwardRule>,
}

fn default_config_empty_string_function() -> String {
    "".to_string()
}
fn default_config_quiet_function() -> bool {
    false
}
fn default_config_false_function() -> bool {
    false
}
fn default_config_log_dir_function() -> String {
    "logs".to_string()
}
fn default_config_transport_max_connections_function() -> u32 {
    16
}
fn default_config_transport_idle_timeout_secs_function() -> u32 {
    60
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
    let mut config: Config = match serde_json::from_reader(reader) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Failed to parse config file: {}", e);
            std::process::exit(1);
        }
    };

    config.forward_rules = crate::global::forward_rule::parse_config(&config);

    config
});
