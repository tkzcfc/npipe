use dashmap::DashMap;
use once_cell::sync::Lazy;
use std::time::{Duration, Instant};

/// 统计失败次数的滑动窗口
const WINDOW: Duration = Duration::from_secs(5 * 60);
/// 窗口内允许的最大失败次数，达到即锁定
const MAX_FAILURES: usize = 5;
/// 锁定时长
const LOCKOUT: Duration = Duration::from_secs(15 * 60);
/// 条目数量上限，超过则清理过期项，避免被轮换 key 撑爆内存
const MAX_ENTRIES: usize = 10_000;

struct Attempts {
    failures: Vec<Instant>,
    locked_until: Option<Instant>,
}

static LOGIN_ATTEMPTS: Lazy<DashMap<String, Attempts>> = Lazy::new(DashMap::new);

/// 若该 key 处于锁定期，返回剩余锁定秒数
pub fn locked_secs_remaining(key: &str) -> Option<u64> {
    let now = Instant::now();
    let entry = LOGIN_ATTEMPTS.get(key)?;
    match entry.locked_until {
        Some(until) if until > now => Some((until - now).as_secs() + 1),
        _ => None,
    }
}

/// 记录一次登录失败，窗口内累计达到阈值则进入锁定
pub fn record_failure(key: &str) {
    let now = Instant::now();

    if LOGIN_ATTEMPTS.len() > MAX_ENTRIES {
        LOGIN_ATTEMPTS.retain(|_, a| {
            a.locked_until.map(|u| u > now).unwrap_or(false) || !a.failures.is_empty()
        });
    }

    let mut entry = LOGIN_ATTEMPTS
        .entry(key.to_owned())
        .or_insert_with(|| Attempts {
            failures: Vec::new(),
            locked_until: None,
        });

    entry.failures.retain(|t| now.duration_since(*t) < WINDOW);
    entry.failures.push(now);
    if entry.failures.len() >= MAX_FAILURES {
        entry.locked_until = Some(now + LOCKOUT);
        entry.failures.clear();
    }
}

/// 登录成功，清除该 key 的失败计数
pub fn record_success(key: &str) {
    LOGIN_ATTEMPTS.remove(key);
}
