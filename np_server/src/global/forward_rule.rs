use crate::global::config::{Config, ForwardRuleConfig};
use log::warn;
use regex::Regex;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

/// 非法流量嗅探等待上限：自首个非 npipe（非 0x21）字节起算，到期必须唤醒读取循环。
/// 不会在后续每个字节到达时刷新。
pub const ILLEGAL_TRAFFIC_SNIFF_TIMEOUT: Duration = Duration::from_millis(1000);

/// 单连接嗅探阶段累计缓冲上限。超过后不再为判定继续读取。
pub const ILLEGAL_TRAFFIC_SNIFF_MAX_BYTES: usize = 1024;

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

/// 单条规则对当前已读缓冲的增量判定结果。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatchResult {
    /// 当前缓冲已足够认定命中。
    Match,
    /// 当前缓冲已足够认定不命中（含已读字节与目标矛盾）。
    Mismatch,
    /// 已读字节仍可能命中，需要继续读取。不得据此让后序规则抢先。
    NeedMore,
}

/// 按规则数组顺序选择转发目标。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectResult {
    /// 命中该规则下标（数组序，先匹配先得）。
    Selected(usize),
    /// 至少一条更靠前的规则仍为 NeedMore，后序 any 不得接走。
    NeedMore,
    /// 所有规则均已确定不匹配，且没有可用的 any。
    None,
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

/// 对单条规则做增量匹配。
///
/// `regex` 没有通用的部分匹配算法：只对当前缓冲做 lossy UTF-8 后立即判定
/// 命中或未命中，短缓冲可能出现假阴性。这是兼容策略，不是“读到完整语义”。
pub fn match_rule(matcher: &Matcher, buf: &[u8]) -> MatchResult {
    match matcher {
        Matcher::IsHttp => match_http(buf),
        Matcher::IsTls => match_tls(buf),
        Matcher::BytesPrefix(prefix) => match_bytes_prefix(prefix, buf),
        Matcher::Regex(re) => {
            let text = String::from_utf8_lossy(buf);
            if re.is_match(&text) {
                MatchResult::Match
            } else {
                MatchResult::Mismatch
            }
        }
        Matcher::Any => MatchResult::Match,
    }
}

fn match_bytes_prefix(prefix: &[u8], buf: &[u8]) -> MatchResult {
    if prefix.is_empty() {
        return MatchResult::Match;
    }
    if buf.len() >= prefix.len() {
        if buf.starts_with(prefix) {
            MatchResult::Match
        } else {
            MatchResult::Mismatch
        }
    } else if prefix.starts_with(buf) {
        MatchResult::NeedMore
    } else {
        MatchResult::Mismatch
    }
}

fn match_http(buf: &[u8]) -> MatchResult {
    if buf.is_empty() {
        return MatchResult::NeedMore;
    }
    if HTTP_METHODS.iter().any(|m| buf.starts_with(m)) {
        return MatchResult::Match;
    }
    if HTTP_METHODS.iter().any(|m| m.starts_with(buf)) {
        return MatchResult::NeedMore;
    }
    MatchResult::Mismatch
}

/// 与历史实现保持一致：至少 3 字节，且前两字节为 `0x16 0x03`（第三字节不校验）。
fn match_tls(buf: &[u8]) -> MatchResult {
    if buf.is_empty() {
        return MatchResult::NeedMore;
    }
    if buf[0] != 0x16 {
        return MatchResult::Mismatch;
    }
    if buf.len() == 1 {
        return MatchResult::NeedMore;
    }
    if buf[1] != 0x03 {
        return MatchResult::Mismatch;
    }
    if buf.len() < 3 {
        return MatchResult::NeedMore;
    }
    MatchResult::Match
}

/// 按数组顺序选择规则。前面规则仍为 NeedMore 时，后面的命中（含 any）不能抢先。
pub fn select_forward_rule(rules: &[ForwardRule], buf: &[u8]) -> SelectResult {
    let mut pending = false;
    for (idx, rule) in rules.iter().enumerate() {
        match match_rule(&rule.matcher, buf) {
            MatchResult::Match => {
                if !pending {
                    return SelectResult::Selected(idx);
                }
            }
            MatchResult::NeedMore => pending = true,
            MatchResult::Mismatch => {}
        }
    }
    if pending {
        SelectResult::NeedMore
    } else {
        SelectResult::None
    }
}

/// 超时、缓冲上限或嗅探阶段读到 EOF 时：先再尝试一次完整选择；
/// 若仍未决或全不匹配，则交给第一条 `any`（不把半截前缀当成命中）。
/// 没有 `any` 则返回 None，由调用方关闭连接。
pub fn resolve_unsatisfied_sniff(rules: &[ForwardRule], buf: &[u8]) -> SelectResult {
    match select_forward_rule(rules, buf) {
        SelectResult::Selected(idx) => SelectResult::Selected(idx),
        SelectResult::NeedMore | SelectResult::None => {
            if let Some(idx) = rules
                .iter()
                .position(|rule| matches!(rule.matcher, Matcher::Any))
            {
                SelectResult::Selected(idx)
            } else {
                SelectResult::None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(matcher: Matcher, port: u16) -> ForwardRule {
        ForwardRule {
            matcher,
            target: SocketAddr::from(([127, 0, 0, 1], port)),
        }
    }

    fn prefix_rule(bytes: &[u8], port: u16) -> ForwardRule {
        rule(Matcher::BytesPrefix(bytes.to_vec()), port)
    }

    fn regex_rule(pattern: &str, port: u16) -> ForwardRule {
        rule(Matcher::Regex(Arc::new(Regex::new(pattern).unwrap())), port)
    }

    /// 测试用假前缀：0x00 + 16 字节假 UUID，不是生产凭证。
    fn vless_prefix() -> Vec<u8> {
        let mut v = vec![0x00];
        v.extend_from_slice(&[
            0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd,
            0xee, 0xff,
        ]);
        assert_eq!(v.len(), 17);
        v
    }

    #[test]
    fn prefix_oneshot_17_matches() {
        let p = vless_prefix();
        let mut buf = p.clone();
        buf.extend_from_slice(b"\x00\x01REST");
        assert_eq!(
            match_rule(&Matcher::BytesPrefix(p), &buf),
            MatchResult::Match
        );
    }

    #[test]
    fn prefix_byte_by_byte_needs_more_then_matches() {
        let p = vless_prefix();
        for n in 1..17 {
            assert_eq!(
                match_rule(&Matcher::BytesPrefix(p.clone()), &p[..n]),
                MatchResult::NeedMore,
                "len={n}"
            );
        }
        assert_eq!(
            match_rule(&Matcher::BytesPrefix(p.clone()), &p),
            MatchResult::Match
        );
    }

    #[test]
    fn prefix_early_mismatch_does_not_wait() {
        let p = vless_prefix();
        assert_eq!(
            match_rule(&Matcher::BytesPrefix(p.clone()), &[0x01]),
            MatchResult::Mismatch
        );
        let mut wrong = p.clone();
        wrong[8] ^= 0xff;
        assert_eq!(
            match_rule(&Matcher::BytesPrefix(p), &wrong[..9]),
            MatchResult::Mismatch
        );
    }

    #[test]
    fn shared_prefix_multi_rule_waits_then_splits() {
        let a = b"\x00\x11\x22\x33AAAA".to_vec();
        let b = b"\x00\x11\x22\x33BBBB".to_vec();
        let rules = vec![
            prefix_rule(&a, 1),
            prefix_rule(&b, 2),
            rule(Matcher::Any, 9),
        ];
        let common = &a[..4];
        assert_eq!(
            select_forward_rule(&rules, common),
            SelectResult::NeedMore,
            "common prefix must not fall through to any"
        );
        assert_eq!(select_forward_rule(&rules, &a), SelectResult::Selected(0));
        assert_eq!(select_forward_rule(&rules, &b), SelectResult::Selected(1));
    }

    #[test]
    fn any_before_prefix_wins_immediately() {
        let p = vless_prefix();
        let rules = vec![rule(Matcher::Any, 9), prefix_rule(&p, 1)];
        assert_eq!(select_forward_rule(&rules, &p), SelectResult::Selected(0));
    }

    #[test]
    fn any_after_prefix_is_blocked_while_need_more() {
        let p = vless_prefix();
        let rules = vec![
            prefix_rule(&p, 1),
            rule(Matcher::IsHttp, 80),
            rule(Matcher::Any, 9),
        ];
        assert_eq!(select_forward_rule(&rules, &p[..8]), SelectResult::NeedMore);
        assert_eq!(select_forward_rule(&rules, &p), SelectResult::Selected(0));
        assert_eq!(
            select_forward_rule(&rules, b"GET / HTTP/1.1\r\n\r\n"),
            SelectResult::Selected(1)
        );
        assert_eq!(
            select_forward_rule(&rules, b"\x16\x03\x01"),
            SelectResult::Selected(2)
        );
    }

    #[test]
    fn http_split_method_needs_more() {
        assert_eq!(match_rule(&Matcher::IsHttp, b"GE"), MatchResult::NeedMore);
        assert_eq!(match_rule(&Matcher::IsHttp, b"GET /"), MatchResult::Match);
        assert_eq!(match_rule(&Matcher::IsHttp, b"XYZ"), MatchResult::Mismatch);
        let p = vless_prefix();
        let rules = vec![
            prefix_rule(&p, 1),
            rule(Matcher::IsHttp, 80),
            rule(Matcher::Any, 9),
        ];
        assert_eq!(
            select_forward_rule(&rules, b"GE"),
            SelectResult::NeedMore,
            "GET split must not go to any"
        );
        assert_eq!(
            select_forward_rule(&rules, b"GET / HTTP/1.1\r\n"),
            SelectResult::Selected(1)
        );
    }

    #[test]
    fn tls_split_and_mismatch() {
        assert_eq!(match_rule(&Matcher::IsTls, b"\x16"), MatchResult::NeedMore);
        assert_eq!(
            match_rule(&Matcher::IsTls, b"\x16\x03"),
            MatchResult::NeedMore
        );
        assert_eq!(
            match_rule(&Matcher::IsTls, b"\x16\x03\x01"),
            MatchResult::Match
        );
        assert_eq!(
            match_rule(&Matcher::IsTls, b"\x16\x02\x01"),
            MatchResult::Mismatch
        );
        assert_eq!(
            match_rule(&Matcher::IsTls, b"\x17\x03"),
            MatchResult::Mismatch
        );
    }

    #[test]
    fn regex_evaluates_current_buffer_only() {
        // Compatible policy: short "GE" does not match ^GET , so it is a mismatch
        // and will fall through (unless an earlier rule is NeedMore).
        let rules = vec![regex_rule("^GET ", 80), rule(Matcher::Any, 9)];
        assert_eq!(match_rule(&rules[0].matcher, b"GE"), MatchResult::Mismatch);
        assert_eq!(
            select_forward_rule(&rules, b"GE"),
            SelectResult::Selected(1),
            "regex must not claim a general partial match"
        );
        assert_eq!(
            select_forward_rule(&rules, b"GET /"),
            SelectResult::Selected(0)
        );
    }

    #[test]
    fn resolve_timeout_does_not_count_as_prefix_hit() {
        let p = vless_prefix();
        let rules = vec![prefix_rule(&p, 1), rule(Matcher::Any, 9)];
        let partial = &p[..8];
        assert_eq!(select_forward_rule(&rules, partial), SelectResult::NeedMore);
        assert_eq!(
            resolve_unsatisfied_sniff(&rules, partial),
            SelectResult::Selected(1),
            "timeout/limit/EOF fallback must use any, not the prefix rule"
        );
    }

    #[test]
    fn resolve_without_any_is_none() {
        let p = vless_prefix();
        let rules = vec![prefix_rule(&p, 1)];
        assert_eq!(
            resolve_unsatisfied_sniff(&rules, &p[..8]),
            SelectResult::None
        );
    }

    #[test]
    fn resolve_prefers_real_match_if_already_decided() {
        let p = vless_prefix();
        let rules = vec![prefix_rule(&p, 1), rule(Matcher::Any, 9)];
        assert_eq!(
            resolve_unsatisfied_sniff(&rules, &p),
            SelectResult::Selected(0)
        );
    }

    #[test]
    fn empty_buffer_prefix_needs_more() {
        let p = vless_prefix();
        assert_eq!(
            match_rule(&Matcher::BytesPrefix(p), b""),
            MatchResult::NeedMore
        );
    }
}
