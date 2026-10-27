use std::sync::{Arc, OnceLock};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

/// 进程内同时存在的监听会话上限。服务端可在首次接受连接前覆盖。
pub const DEFAULT_MAX_INFLIGHT_CONNECTIONS: usize = 4096;

static MAX_INFLIGHT: OnceLock<Arc<Semaphore>> = OnceLock::new();

pub fn set_max_inflight_connections(max: usize) {
    let max = max.max(1);
    let _ = MAX_INFLIGHT.set(Arc::new(Semaphore::new(max)));
}

fn semaphore() -> Arc<Semaphore> {
    MAX_INFLIGHT
        .get_or_init(|| Arc::new(Semaphore::new(DEFAULT_MAX_INFLIGHT_CONNECTIONS)))
        .clone()
}

/// 拿到许可才接受这条连接。许可要一直持有到会话结束。
pub fn try_acquire_connection() -> Option<OwnedSemaphorePermit> {
    semaphore().try_acquire_owned().ok()
}
