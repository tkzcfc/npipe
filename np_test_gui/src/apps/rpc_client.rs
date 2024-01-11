use anyhow::anyhow;
use byteorder::{BigEndian, ByteOrder};
use bytes::BytesMut;
use log::error;
use np_base::net::client::Client;
use np_proto::message_map::{
    decode_message, encode_raw_message, get_message_id, get_message_size, MessageType,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::time::Instant;

pub type RecvMessageCallback = Box<dyn FnMut(i32, &MessageType) + Send + 'static>;
pub type RequestResultCallback = Box<dyn FnMut(anyhow::Result<&MessageType>) + Send + 'static>;

pub(crate) struct RpcClient {
    inner: Client,
    serial: i32,
    on_recv_message_callback: Option<RecvMessageCallback>,
    response_map: HashMap<i32, (RequestResultCallback, Instant)>,
    last_clear_time: Instant,
}

impl RpcClient {
    pub(crate) fn new(addr: SocketAddr) -> Self {
        RpcClient {
            inner: Client::new(addr, try_extract_frame),
            serial: 0,
            on_recv_message_callback: None,
            response_map: HashMap::new(),
            last_clear_time: Instant::now(),
        }
    }

    #[inline]
    pub(crate) async fn connect(&mut self) -> anyhow::Result<()> {
        self.inner.connect().await
    }

    #[inline]
    pub fn reset_addr(&mut self, addr: SocketAddr) {
        self.inner.reset_addr(addr)
    }

    #[inline]
    pub(crate) fn is_connect(&self) -> bool {
        self.inner.is_connect()
    }

    #[inline]
    pub(crate) fn disconnect(&mut self) {
        self.inner.disconnect()
    }

    pub(crate) fn set_recv_message_callback<F>(&mut self, callback: F)
    where
        F: FnMut(i32, &MessageType) + 'static + Send,
    {
        self.on_recv_message_callback = Some(Box::new(callback));
    }

    pub(crate) fn send_request<F>(&mut self, message: MessageType, callback: F) -> bool
    where
        F: FnMut(anyhow::Result<&MessageType>) + 'static + Send,
    {
        if !self.is_connect() {
            error!("not connected");
            return false;
        }
        // 防止请求序号越界
        if self.serial >= i32::MAX {
            self.serial = 0;
        }
        self.serial += 1;
        let serial = -self.serial;

        if let Err(error) = self.package_and_send_message(serial, &message) {
            error!("Send message error: {}", error);
            return false;
        }

        self.response_map
            .insert(serial, (Box::new(callback), Instant::now()));

        true
    }

    #[inline]
    pub(crate) fn package_and_send_message(
        &self,
        serial: i32,
        message: &MessageType,
    ) -> anyhow::Result<()> {
        if let Some(message_id) = get_message_id(message) {
            let message_size = get_message_size(message);
            let mut buf = Vec::with_capacity(message_size + 12);

            byteorder::WriteBytesExt::write_u32::<BigEndian>(&mut buf, (8 + message_size) as u32)?;
            byteorder::WriteBytesExt::write_i32::<BigEndian>(&mut buf, serial)?;
            byteorder::WriteBytesExt::write_u32::<BigEndian>(&mut buf, message_id)?;
            encode_raw_message(message, &mut buf);

            return self.inner.send(buf, true);
        }
        Err(anyhow!("error message"))
    }

    pub(crate) fn update(&mut self) {
        if !self.inner.is_connect() {
            return;
        }
        loop {
            if let Some(frame) = self.inner.try_recv() {
                // 消息不合法，长度不够
                if frame.len() < 8 {
                    self.disconnect();

                    for (ref mut callback, ref _time) in self.response_map.values_mut() {
                        callback(Err(anyhow!("The response is illegal")))
                    }
                    self.response_map.clear();
                    break;
                }
                // 消息序号
                let mut serial: i32 = BigEndian::read_i32(&frame[0..4]);
                // 消息类型id
                let msg_id: u32 = BigEndian::read_u32(&frame[4..8]);

                match decode_message(msg_id, &frame[8..]) {
                    Ok(message) => {
                        if let Some(ref mut on_recv_message_callback) =
                            self.on_recv_message_callback
                        {
                            on_recv_message_callback(serial, &message);
                        }

                        serial = -serial;
                        if let Some((ref mut callback, ref _time)) =
                            self.response_map.get_mut(&serial)
                        {
                            callback(Ok(&message));
                            self.response_map.remove(&serial);
                        }
                    }
                    Err(err) => {
                        error!("Protobuf parse error: {}", err);
                        serial = -serial;

                        if let Some((ref mut callback, ref _time)) =
                            self.response_map.get_mut(&serial)
                        {
                            callback(Err(err.into()));
                            self.response_map.remove(&serial);
                        }
                        break;
                    }
                }
            } else {
                break;
            }
        }

        let now = Instant::now();
        if now.duration_since(self.last_clear_time) > Duration::from_secs(1) {
            let duration = Duration::from_secs(15);
            self.response_map.retain(|_, (callback, time)| {
                if now.duration_since(*time) < duration {
                    true
                } else {
                    callback(Err(anyhow!("timeout")));
                    false
                }
            });
        }
    }
}

// 数据粘包处理
#[inline]
fn try_extract_frame(buffer: &mut BytesMut) -> anyhow::Result<Option<Vec<u8>>> {
    // 数据小于4字节
    if buffer.len() < 4 {
        return Ok(None);
    }

    let bin = buffer.get(0..4).unwrap();
    let len = BigEndian::read_u32(bin) as usize;

    // 超出最大限制
    if len <= 0 || len >= 1024 * 1024 * 5 {
        return Err(anyhow!("bad length"));
    }

    // 数据不够
    if buffer.len() < 4 + len {
        return Ok(None);
    }

    let frame = buffer.split_to(4 + len).split_off(4).to_vec();

    Ok(Some(frame))
}
