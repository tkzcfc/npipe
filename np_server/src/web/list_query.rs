//! Shared validation and matching helpers for player_list / tunnel_list.

use sea_orm::sea_query::{Expr, SimpleExpr};

/// Maximum length of a trimmed search string, counted in Unicode scalar values.
pub const MAX_QUERY_CHARS: usize = 64;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlayerListQuery {
    pub q: Option<String>,
    pub enabled: Option<u8>,
    pub web_access: Option<u8>,
    pub online: Option<u8>,
}

impl PlayerListQuery {
    pub fn has_filter(&self) -> bool {
        self.q.is_some()
            || self.enabled.is_some()
            || self.web_access.is_some()
            || self.online.is_some()
    }

    pub fn needs_memory_scan(&self) -> bool {
        self.online.is_some()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UserRoleFilter {
    Any,
    Sender,
    Receiver,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TunnelListQuery {
    pub q: Option<String>,
    pub tunnel_type: Option<u32>,
    pub enabled: Option<u8>,
    pub user_id: Option<u32>,
    pub user_role: Option<UserRoleFilter>,
    pub listen_status: Option<String>,
}

impl TunnelListQuery {
    pub fn has_filter(&self) -> bool {
        self.q.is_some()
            || self.tunnel_type.is_some()
            || self.enabled.is_some()
            || self.user_id.is_some()
            || self.user_role.is_some()
            || self.listen_status.is_some()
    }
}

pub fn parse_q(value: &Option<String>) -> Result<Option<String>, String> {
    let Some(raw) = value.as_ref() else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.chars().count() > MAX_QUERY_CHARS {
        return Err("q exceeds 64 characters".into());
    }
    Ok(Some(trimmed.to_owned()))
}

pub fn parse_flag(name: &str, value: Option<i64>) -> Result<Option<u8>, String> {
    match value {
        None => Ok(None),
        Some(0) => Ok(Some(0)),
        Some(1) => Ok(Some(1)),
        Some(_) => Err(format!("invalid {name}, expected 0 or 1")),
    }
}

/// Tunnel types follow `InletProxyType`: 0=TCP, 1=UDP, 2=SOCKS5, 3=HTTP.
pub fn parse_tunnel_type(value: Option<i64>) -> Result<Option<u32>, String> {
    match value {
        None => Ok(None),
        Some(v) if (0..=3).contains(&v) => Ok(Some(v as u32)),
        Some(_) => Err("invalid tunnel_type, expected 0=TCP, 1=UDP, 2=SOCKS5, 3=HTTP".into()),
    }
}

pub fn parse_user_role(value: &Option<String>) -> Result<Option<UserRoleFilter>, String> {
    let Some(raw) = value.as_ref() else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    match trimmed.to_ascii_lowercase().as_str() {
        "any" => Ok(Some(UserRoleFilter::Any)),
        "sender" => Ok(Some(UserRoleFilter::Sender)),
        "receiver" => Ok(Some(UserRoleFilter::Receiver)),
        _ => Err("invalid user_role, expected any, sender, or receiver".into()),
    }
}

pub fn parse_listen_status(value: &Option<String>) -> Result<Option<String>, String> {
    let Some(raw) = value.as_ref() else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let normalized = trimmed.to_ascii_lowercase();
    match normalized.as_str() {
        "disabled" | "offline" | "listening" | "failed" | "unknown" => Ok(Some(normalized)),
        _ => Err(
            "invalid listen_status, expected disabled, offline, listening, failed, or unknown"
                .into(),
        ),
    }
}

pub fn parse_player_list_query(
    q: &Option<String>,
    enabled: Option<i64>,
    web_access: Option<i64>,
    online: Option<i64>,
) -> Result<PlayerListQuery, String> {
    Ok(PlayerListQuery {
        q: parse_q(q)?,
        enabled: parse_flag("enabled", enabled)?,
        web_access: parse_flag("web_access", web_access)?,
        online: parse_flag("online", online)?,
    })
}

pub fn parse_tunnel_list_query(
    q: &Option<String>,
    tunnel_type: Option<i64>,
    enabled: Option<i64>,
    user_id: Option<u32>,
    user_role: &Option<String>,
    listen_status: &Option<String>,
) -> Result<TunnelListQuery, String> {
    let parsed = TunnelListQuery {
        q: parse_q(q)?,
        tunnel_type: parse_tunnel_type(tunnel_type)?,
        enabled: parse_flag("enabled", enabled)?,
        user_id,
        user_role: parse_user_role(user_role)?,
        listen_status: parse_listen_status(listen_status)?,
    };
    if parsed.user_role.is_some() && parsed.user_id.is_none() {
        return Err("user_role requires user_id".into());
    }
    Ok(parsed)
}

pub fn normalize_page_size(page_size: usize) -> usize {
    if page_size == 0 {
        20
    } else {
        page_size.min(100)
    }
}

pub fn page_range(page_number: usize, page_size: usize, total: usize) -> (usize, usize) {
    let start = page_number.saturating_mul(page_size);
    if start >= total {
        (start, start)
    } else {
        (start, start.saturating_add(page_size).min(total))
    }
}

pub fn slice_page<T: Clone>(items: &[T], page_number: usize, page_size: usize) -> Vec<T> {
    let (start, end) = page_range(page_number, page_size, items.len());
    if start >= end {
        Vec::new()
    } else {
        items[start..end].to_vec()
    }
}

/// Escape `\`, `%`, and `_` so they are treated as literals in SQL LIKE.
pub fn escape_like(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\\' | '%' | '_' => {
                out.push('\\');
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    out
}

pub fn username_contains_expr(q: &str) -> SimpleExpr {
    let pattern = format!("%{}%", escape_like(&q.to_ascii_lowercase()));
    Expr::cust_with_values("LOWER(username) LIKE ? ESCAPE '\\'", [pattern])
}

pub fn parse_digit_id(q: &str) -> Option<u32> {
    if q.is_empty() || !q.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    q.parse().ok()
}

pub fn text_contains_ci(haystack: &str, needle: &str) -> bool {
    haystack
        .to_ascii_lowercase()
        .contains(&needle.to_ascii_lowercase())
}

#[allow(dead_code)]
pub fn player_matches_q(id: u32, username: &str, q: &str) -> bool {
    if text_contains_ci(username, q) {
        return true;
    }
    parse_digit_id(q).is_some_and(|parsed| parsed == id)
}

#[derive(Clone, Debug)]
pub struct TunnelMatchInput<'a> {
    pub id: u32,
    pub source: &'a str,
    pub endpoint: &'a str,
    pub description: &'a str,
    pub tunnel_type: u32,
    pub enabled: u8,
    pub sender: u32,
    pub receiver: u32,
    pub sender_name: &'a str,
    pub receiver_name: &'a str,
    pub listen_status: &'a str,
}

pub fn user_is_associated(
    sender: u32,
    receiver: u32,
    user_id: u32,
    role: UserRoleFilter,
) -> bool {
    match role {
        UserRoleFilter::Any => sender == user_id || receiver == user_id,
        UserRoleFilter::Sender => sender == user_id,
        UserRoleFilter::Receiver => receiver == user_id,
    }
}

/// q matches description, source, endpoint, associated usernames, and related numeric IDs.
/// It never matches proxy auth username or password.
pub fn tunnel_matches_q(item: &TunnelMatchInput<'_>, q: &str) -> bool {
    if text_contains_ci(item.description, q)
        || text_contains_ci(item.source, q)
        || text_contains_ci(item.endpoint, q)
        || text_contains_ci(item.sender_name, q)
        || text_contains_ci(item.receiver_name, q)
    {
        return true;
    }
    if let Some(id) = parse_digit_id(q) {
        return item.id == id || item.sender == id || item.receiver == id;
    }
    false
}

pub fn tunnel_matches(item: &TunnelMatchInput<'_>, query: &TunnelListQuery) -> bool {
    if let Some(q) = query.q.as_deref() {
        if !tunnel_matches_q(item, q) {
            return false;
        }
    }
    if let Some(tunnel_type) = query.tunnel_type {
        if item.tunnel_type != tunnel_type {
            return false;
        }
    }
    if let Some(enabled) = query.enabled {
        if item.enabled != enabled {
            return false;
        }
    }
    if let Some(user_id) = query.user_id {
        let role = query.user_role.unwrap_or(UserRoleFilter::Any);
        if !user_is_associated(item.sender, item.receiver, user_id, role) {
            return false;
        }
    }
    if let Some(status) = query.listen_status.as_deref() {
        if item.listen_status != status {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_q_trims_and_rejects_overlong() {
        assert_eq!(parse_q(&None).unwrap(), None);
        assert_eq!(parse_q(&Some("  ".into())).unwrap(), None);
        assert_eq!(parse_q(&Some("  alice  ".into())).unwrap(), Some("alice".into()));
        let long = "a".repeat(65);
        assert!(parse_q(&Some(long)).unwrap_err().contains("64"));
        let exact = "b".repeat(64);
        assert_eq!(parse_q(&Some(exact.clone())).unwrap(), Some(exact));
        let unicode = "测".repeat(64);
        assert_eq!(parse_q(&Some(unicode.clone())).unwrap(), Some(unicode));
        assert!(parse_q(&Some("测".repeat(65))).is_err());
    }

    #[test]
    fn flags_and_enums_validate() {
        assert_eq!(parse_flag("enabled", None).unwrap(), None);
        assert_eq!(parse_flag("enabled", Some(0)).unwrap(), Some(0));
        assert_eq!(parse_flag("online", Some(1)).unwrap(), Some(1));
        assert!(parse_flag("online", Some(2)).is_err());
        assert_eq!(parse_tunnel_type(Some(0)).unwrap(), Some(0));
        assert_eq!(parse_tunnel_type(Some(3)).unwrap(), Some(3));
        assert!(parse_tunnel_type(Some(4)).is_err());
        assert!(parse_tunnel_type(Some(-1)).is_err());
        assert_eq!(parse_user_role(&Some("Sender".into())).unwrap(), Some(UserRoleFilter::Sender));
        assert_eq!(parse_user_role(&Some("  ".into())).unwrap(), None);
        assert!(parse_user_role(&Some("owner".into())).is_err());
        assert_eq!(
            parse_listen_status(&Some("Listening".into())).unwrap(),
            Some("listening".into())
        );
        assert!(parse_listen_status(&Some("up".into())).is_err());
    }

    #[test]
    fn user_role_requires_user_id() {
        let err = parse_tunnel_list_query(
            &None,
            None,
            None,
            None,
            &Some("sender".into()),
            &None,
        )
        .unwrap_err();
        assert!(err.contains("user_role requires user_id"));
        let ok = parse_tunnel_list_query(
            &None,
            None,
            None,
            Some(7),
            &Some("receiver".into()),
            &None,
        )
        .unwrap();
        assert_eq!(ok.user_id, Some(7));
        assert_eq!(ok.user_role, Some(UserRoleFilter::Receiver));
    }

    #[test]
    fn page_size_and_overflow() {
        assert_eq!(normalize_page_size(0), 20);
        assert_eq!(normalize_page_size(20), 20);
        assert_eq!(normalize_page_size(100), 100);
        assert_eq!(normalize_page_size(1000), 100);
        let items: Vec<u32> = (1..=25).collect();
        assert_eq!(slice_page(&items, 0, 20), (1..=20).collect::<Vec<_>>());
        assert_eq!(slice_page(&items, 1, 20), vec![21, 22, 23, 24, 25]);
        assert!(slice_page(&items, 2, 20).is_empty());
        // overflow should not panic and should yield an empty page
        let empty = slice_page(&items, usize::MAX, 20);
        assert!(empty.is_empty());
        let (start, end) = page_range(usize::MAX, 100, 25);
        assert!(start >= end || start >= 25);
    }

    #[test]
    fn escape_like_treats_wildcards_as_literals() {
        assert_eq!(escape_like("a%b_c\\d"), r"a\%b\_c\\d");
        assert_eq!(escape_like("plain"), "plain");
    }

    #[test]
    fn player_q_matches_username_or_exact_id() {
        assert!(player_matches_q(12, "Alice", "ali"));
        assert!(player_matches_q(12, "Alice", "12"));
        assert!(!player_matches_q(12, "Alice", "13"));
        assert!(!player_matches_q(12, "Alice", "bob"));
        // mixed alphanumeric is username-only, not parsed as id
        assert!(!player_matches_q(12, "Alice", "12a"));
    }

    #[test]
    fn tunnel_q_does_not_search_proxy_auth_or_password() {
        let item = TunnelMatchInput {
            id: 9,
            source: "127.0.0.1:7000",
            endpoint: "10.0.0.2:80",
            description: "office-gateway",
            tunnel_type: 3,
            enabled: 1,
            sender: 4,
            receiver: 5,
            sender_name: "alice",
            receiver_name: "bob",
            listen_status: "listening",
        };
        assert!(tunnel_matches_q(&item, "office"));
        assert!(tunnel_matches_q(&item, "7000"));
        assert!(tunnel_matches_q(&item, "alice"));
        assert!(tunnel_matches_q(&item, "9"));
        assert!(tunnel_matches_q(&item, "5"));
        // proxy auth / password must not be searchable even if they happen to equal q
        let secret_item = TunnelMatchInput {
            sender_name: "alice",
            receiver_name: "bob",
            description: "plain",
            source: "1.1.1.1:1",
            endpoint: "2.2.2.2:2",
            ..item
        };
        assert!(!tunnel_matches_q(&secret_item, "s3cret-password"));
        assert!(!tunnel_matches_q(&secret_item, "proxy-user"));
    }

    #[test]
    fn tunnel_filters_combine_with_and() {
        let item = TunnelMatchInput {
            id: 1,
            source: "127.0.0.1:8000",
            endpoint: "10.0.0.1:443",
            description: "edge",
            tunnel_type: 0,
            enabled: 1,
            sender: 2,
            receiver: 3,
            sender_name: "alice",
            receiver_name: "bob",
            listen_status: "listening",
        };
        let mut query = TunnelListQuery {
            q: Some("edge".into()),
            tunnel_type: Some(0),
            enabled: Some(1),
            user_id: Some(2),
            user_role: Some(UserRoleFilter::Sender),
            listen_status: Some("listening".into()),
        };
        assert!(tunnel_matches(&item, &query));
        query.tunnel_type = Some(2);
        assert!(!tunnel_matches(&item, &query));
        query.tunnel_type = Some(0);
        query.user_role = Some(UserRoleFilter::Receiver);
        assert!(!tunnel_matches(&item, &query));
        query.user_role = Some(UserRoleFilter::Any);
        query.user_id = Some(3);
        assert!(tunnel_matches(&item, &query));
        query.listen_status = Some("failed".into());
        assert!(!tunnel_matches(&item, &query));
    }
}
