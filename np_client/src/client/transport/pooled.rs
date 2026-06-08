use anyhow::anyhow;
use log::{debug, info};
use np_proto::client_server::BindTransportReq;
use np_proto::message_map::{get_message_size, MessageType};
use np_proto::utils::message_bridge;
use np_proto::utils::transport::TRANSPORT_CONNECTION_TYPE_FORWARD;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio::sync::Mutex;
use tokio::time::sleep;

use super::inner::{PooledForwardPath, PooledTransportState};
use crate::client::io::{package_and_send_message, read_transport_events};
use crate::client::now_secs;

impl<S> PooledForwardPath<S>
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    fn load_score(&self) -> usize {
        self.active_sessions.load(Ordering::Relaxed) * 1_000_000
            + self.inflight_bytes.load(Ordering::Relaxed)
    }

    fn bind_session(&self) {
        self.active_sessions.fetch_add(1, Ordering::Relaxed);
        self.last_used_secs.store(now_secs(), Ordering::Relaxed);
    }

    fn unbind_session(&self) {
        let _ = self
            .active_sessions
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                Some(value.saturating_sub(1))
            });
        self.last_used_secs.store(now_secs(), Ordering::Relaxed);
    }
}

impl<S> PooledTransportState<S>
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    pub async fn configure_from_login(
        &self,
        token: String,
        max_forward_paths: u32,
        idle_timeout_secs: u32,
    ) {
        self.max_forward_paths
            .store(max_forward_paths, Ordering::Relaxed);
        self.idle_timeout_secs
            .store(idle_timeout_secs, Ordering::Relaxed);
        let mut guard = self.token.lock().await;
        *guard = token;
        info!(
            "transport pool configured, max_forward_paths:{}, idle_timeout_secs:{}",
            max_forward_paths, idle_timeout_secs
        );
    }

    pub async fn send_control_message(
        &self,
        serial: i32,
        message: &MessageType,
    ) -> anyhow::Result<()> {
        package_and_send_message(self.control_writer.clone(), serial, message).await
    }

    pub async fn send_proxy_message(
        &self,
        serial: i32,
        message: &MessageType,
    ) -> anyhow::Result<()> {
        let Some(session_id) = message_bridge::pb_proxy_session_id(message) else {
            return self.send_control_message(serial, message).await;
        };

        let Some(path) = self.get_or_create_forward_path(session_id).await? else {
            return self.send_control_message(serial, message).await;
        };

        let message_size = get_message_size(message) + 13;
        path.inflight_bytes
            .fetch_add(message_size, Ordering::Relaxed);
        let result = package_and_send_message(path.writer.clone(), serial, message).await;
        path.inflight_bytes
            .fetch_sub(message_size, Ordering::Relaxed);
        path.last_used_secs.store(now_secs(), Ordering::Relaxed);

        if message_bridge::pb_proxy_is_disconnect(message) {
            self.unbind_session_path(session_id);
        }

        result
    }

    pub fn bind_incoming_message_path(&self, message: &MessageType, path_id: Option<u64>) {
        let (Some(path_id), Some(session_id)) =
            (path_id, message_bridge::pb_proxy_session_id(message))
        else {
            return;
        };

        if let Some(path) = self.forward_paths.get(&path_id).map(|p| p.clone()) {
            if self
                .session_paths
                .insert(session_id, path.clone())
                .is_none()
            {
                path.bind_session();
            }
            if message_bridge::pb_proxy_is_disconnect(message) {
                self.unbind_session_path(session_id);
            }
        }
    }

    async fn get_or_create_forward_path(
        &self,
        session_id: u32,
    ) -> anyhow::Result<Option<Arc<PooledForwardPath<S>>>> {
        let max_paths = self.max_forward_paths.load(Ordering::Relaxed);
        if max_paths == 0 {
            debug!("transport pool disabled, proxy_session_id:{session_id} uses control path");
            return Ok(None);
        }

        let token = self.token.lock().await.clone();
        if token.is_empty() {
            debug!("transport token empty, proxy_session_id:{session_id} uses control path");
            return Ok(None);
        }

        if let Some(path) = self.session_paths.get(&session_id).map(|p| p.clone()) {
            return Ok(Some(path));
        }

        let _guard = self.forward_path_create_lock.lock().await;
        if let Some(path) = self.session_paths.get(&session_id).map(|p| p.clone()) {
            return Ok(Some(path));
        }

        let path = if self.forward_paths.len() < max_paths as usize {
            self.open_forward_path(token).await?
        } else {
            debug!(
                "transport pool reached limit, proxy_session_id:{}, current_paths:{}, max_paths:{}",
                session_id,
                self.forward_paths.len(),
                max_paths
            );
            self.select_least_loaded_path()
                .ok_or_else(|| anyhow!("no available forward connection"))?
        };

        if self
            .session_paths
            .insert(session_id, path.clone())
            .is_none()
        {
            path.bind_session();
        }
        Ok(Some(path))
    }

    async fn open_forward_path(&self, token: String) -> anyhow::Result<Arc<PooledForwardPath<S>>> {
        let connection_id = self.next_connection_id.fetch_add(1, Ordering::Relaxed);
        info!("opening forward transport connection, connection_id:{connection_id}");
        let stream = (self.connector)().await?;
        let (reader, writer) = tokio::io::split(stream);
        let writer = Arc::new(Mutex::new(writer));

        package_and_send_message(
            writer.clone(),
            -3,
            &MessageType::ClientServerBindTransportReq(BindTransportReq {
                transport_token: token,
                connection_id,
                connection_type: TRANSPORT_CONNECTION_TYPE_FORWARD,
            }),
        )
        .await?;

        let path = Arc::new(PooledForwardPath {
            connection_id,
            writer: writer.clone(),
            active_sessions: AtomicUsize::new(0),
            inflight_bytes: AtomicUsize::new(0),
            last_used_secs: AtomicU64::new(now_secs()),
        });

        self.forward_paths.insert(connection_id, path.clone());
        debug!(
            "forward transport connection opened, connection_id:{}, current_paths:{}",
            connection_id,
            self.forward_paths.len()
        );
        tokio::spawn(read_transport_events(
            reader,
            Some(connection_id),
            self.event_tx.clone(),
            self.last_active_secs.clone(),
            self.last_read_secs.clone(),
        ));

        Ok(path)
    }

    fn select_least_loaded_path(&self) -> Option<Arc<PooledForwardPath<S>>> {
        self.forward_paths
            .iter()
            .map(|entry| entry.value().clone())
            .min_by_key(|path| (path.load_score(), path.connection_id))
    }

    fn unbind_session_path(&self, session_id: u32) {
        if let Some((_, path)) = self.session_paths.remove(&session_id) {
            path.unbind_session();
        }
    }

    pub async fn remove_forward_path(&self, connection_id: u64) {
        self.session_paths
            .retain(|_, path| path.connection_id != connection_id);
        if let Some((_, path)) = self.forward_paths.remove(&connection_id) {
            info!(
                "remove forward transport connection, connection_id:{}, remaining_paths:{}",
                connection_id,
                self.forward_paths.len()
            );
            let _ = path.writer.lock().await.shutdown().await;
        }
    }

    async fn close_idle_forward_paths(&self) {
        let idle_timeout_secs = self.idle_timeout_secs.load(Ordering::Relaxed);
        if idle_timeout_secs == 0 {
            return;
        }

        let now = now_secs();
        let connection_ids = self
            .forward_paths
            .iter()
            .filter(|entry| {
                entry.value().active_sessions.load(Ordering::Relaxed) == 0
                    && now.saturating_sub(entry.value().last_used_secs.load(Ordering::Relaxed))
                        >= u64::from(idle_timeout_secs)
            })
            .map(|entry| *entry.key())
            .collect::<Vec<_>>();

        for connection_id in connection_ids {
            if let Some((_, path)) = self.forward_paths.remove(&connection_id) {
                info!(
                    "close idle forward transport connection, connection_id:{}, remaining_paths:{}",
                    connection_id,
                    self.forward_paths.len()
                );
                let _ = path.writer.lock().await.shutdown().await;
            }
        }
    }

    pub fn start_idle_cleanup(state: Arc<Self>) {
        tokio::spawn(async move {
            loop {
                sleep(Duration::from_secs(5)).await;
                if Arc::strong_count(&state) <= 1 {
                    break;
                }
                state.close_idle_forward_paths().await;
            }
        });
    }
}
