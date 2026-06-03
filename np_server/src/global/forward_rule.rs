use crate::global::config::{Config, ForwardRuleConfig};
use log::warn;
use regex::Regex;
use std::net::SocketAddr;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub enum Matcher {
    /// 检测 HTTP 方法开头（GET/POST/PUT 等）
    IsHttp,
    /// 检测 TLS ClientHello (0x16 0x03)
    IsTls,
    /// buffer 前 N 字节包含指定字节序列
    BytesPrefix(Vec<u8>),
    /// 正则表达式匹配
    Regex(Arc<Regex>),
    /// 兜底，永远匹配
    Any,
}

#[derive(Clone, Debug)]
pub struct ForwardRule {
    pub matcher: Matcher,
    pub target: SocketAddr,
}

impl ForwardRule {
    /// 从字符串配置编译成运行时规则
    ///
    /// match_expr 语法：
    ///   "http"            内置 HTTP 方法检测
    ///   "tls"             内置 TLS ClientHello 检测
    ///   "any"             兜底，永远匹配
    ///   "prefix:<hex>"    十六进制字节前缀，如 "prefix:1603"
    ///   "prefix:str:<s>"  字符串前缀，如 "prefix:str:GET "
    ///   "regex:<pattern>" 正则表达式，如 "regex:^(GET|POST) "
    pub fn compile(cfg: &ForwardRuleConfig) -> anyhow::Result<Self> {
        let matcher = parse_match_expr(&cfg.match_expr)?;
        let target = cfg.target.parse::<SocketAddr>()?;
        Ok(ForwardRule { matcher, target })
    }
}

fn parse_match_expr(expr: &str) -> anyhow::Result<Matcher> {
    if expr == "http" {
        return Ok(Matcher::IsHttp);
    }
    if expr == "tls" {
        return Ok(Matcher::IsTls);
    }
    if expr == "any" {
        return Ok(Matcher::Any);
    }
    if let Some(rest) = expr.strip_prefix("prefix:str:") {
        return Ok(Matcher::BytesPrefix(rest.as_bytes().to_vec()));
    }
    if let Some(hex) = expr.strip_prefix("prefix:") {
        let bytes = hex::decode(hex.replace(' ', ""))
            .map_err(|e| anyhow::anyhow!("Invalid hex in prefix: {e}"))?;
        return Ok(Matcher::BytesPrefix(bytes));
    }
    if let Some(pattern) = expr.strip_prefix("regex:") {
        let re =
            Regex::new(pattern).map_err(|e| anyhow::anyhow!("Invalid regex '{pattern}': {e}"))?;
        return Ok(Matcher::Regex(Arc::new(re)));
    }

    anyhow::bail!("Unknown match_expr syntax: '{expr}'\nSupported: http | tls | any | prefix:<hex> | prefix:str:<s> | regex:<pattern>")
}

pub fn parse_config(config: &Config) -> Vec<ForwardRule> {
    let mut rules = config
        .illegal_traffic_forward_rules
        .iter()
        .filter_map(|rule| {
            ForwardRule::compile(rule)
                .map_err(|e| {
                    warn!(
                        "Failed to compile forward rule with match_expr '{}': {e}",
                        rule.match_expr
                    );
                })
                .ok()
        })
        .collect::<Vec<_>>();

    if !config.illegal_traffic_forward.is_empty() {
        match config.illegal_traffic_forward.parse::<SocketAddr>() {
            Ok(target) => rules.push(ForwardRule {
                matcher: Matcher::Any,
                target,
            }),
            Err(_) => {
                warn!(
                    "Invalid illegal_traffic_forward address '{}', skipping",
                    config.illegal_traffic_forward
                );
            }
        }
    }

    rules
}

const HTTP_METHODS: &[&[u8]] = &[
    b"GET ",
    b"POST ",
    b"PUT ",
    b"DELETE ",
    b"HEAD ",
    b"OPTIONS ",
    b"PATCH ",
    b"CONNECT ",
    b"TRACE ",
];

pub fn match_rule(matcher: &Matcher, buf: &[u8]) -> bool {
    match matcher {
        Matcher::IsHttp => HTTP_METHODS.iter().any(|m| buf.starts_with(m)),
        Matcher::IsTls => buf.len() >= 3 && buf[0] == 0x16 && buf[1] == 0x03,
        Matcher::BytesPrefix(prefix) => buf.starts_with(prefix),
        Matcher::Regex(re) => {
            // 对二进制流做 lossy UTF-8 转换后匹配
            let text = String::from_utf8_lossy(buf);
            re.is_match(&text)
        }
        Matcher::Any => true,
    }
}
