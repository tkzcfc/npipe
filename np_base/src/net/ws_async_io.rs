use bytes::Bytes;
use futures::{Sink, Stream};
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};

/// 包装 WebSocketStream，使其实现 `AsyncRead` + `AsyncWrite`
pub struct WebSocketAsyncIo<S> {
    ws_stream: WebSocketStream<S>,
    /// 缓存未读完的 WebSocket 帧数据（Bytes 是引用计数切片，slice 是 O(1)）
    read_buf: Bytes,
}

impl<S> WebSocketAsyncIo<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    pub fn new(ws_stream: WebSocketStream<S>) -> Self {
        Self {
            ws_stream,
            read_buf: Bytes::new(),
        }
    }

    /// 将 data 尽可能写入 buf，多余部分保存在 read_buf
    fn process_data(&mut self, data: Bytes, buf: &mut ReadBuf<'_>) {
        let remaining = buf.remaining();
        if data.len() <= remaining {
            // 整个消息可直接写入
            buf.put_slice(&data);
        } else {
            // 部分写入 buf，剩余用 Bytes::slice O(1) 保留
            buf.put_slice(&data[..remaining]);
            self.read_buf = data.slice(remaining..);
        }
    }
}

impl<S> AsyncRead for WebSocketAsyncIo<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let mut is_read_data_tag = false;
        loop {
            // 优先从缓存读取残留数据（read_buf.slice() 是 O(1)）
            if !self.read_buf.is_empty() {
                is_read_data_tag = true;
                let n = std::cmp::min(self.read_buf.len(), buf.remaining());
                buf.put_slice(&self.read_buf[..n]);
                // O(1): slice 不拷贝数据，只移动内部指针
                self.read_buf = self.read_buf.slice(n..);
            }
            if buf.remaining() == 0 {
                return Poll::Ready(Ok(()));
            }

            match Pin::new(&mut self.ws_stream).poll_next(cx) {
                Poll::Ready(Some(Ok(msg))) => {
                    match msg {
                        Message::Text(text) => {
                            // Utf8Bytes → Bytes: Into<Bytes> 实现是零拷贝（共享同一块内存）
                            self.process_data(Bytes::from(text), buf);
                            is_read_data_tag = true;
                        }
                        Message::Binary(data) => {
                            // tungstenite 0.20+ Binary 已是 Bytes，直接传入无需 to_vec()
                            self.process_data(data, buf);
                            is_read_data_tag = true;
                        }
                        Message::Ping(payload) => {
                            match Pin::new(&mut self.ws_stream).poll_ready(cx) {
                                Poll::Ready(Ok(())) => {
                                    if let Err(e) = Pin::new(&mut self.ws_stream)
                                        .start_send(Message::Pong(payload))
                                    {
                                        return Poll::Ready(Err(io::Error::other(e)));
                                    }
                                }
                                Poll::Ready(Err(e)) => {
                                    return Poll::Ready(Err(io::Error::other(e)));
                                }
                                Poll::Pending => {}
                            }
                        }
                        Message::Close(_) => {
                            return Poll::Ready(Err(io::Error::new(
                                io::ErrorKind::ConnectionAborted,
                                "WebSocket closed",
                            )));
                        }
                        // 丢弃 Pong 消息 和 Frame 消息
                        Message::Frame(_) | Message::Pong(_) => {}
                    }
                }
                Poll::Ready(Some(Err(e))) => {
                    return Poll::Ready(Err(io::Error::other(e.to_string())));
                }
                Poll::Ready(None) => {
                    // 流已关闭。若本轮已填入部分数据，先返回这些字节；
                    // 调用方下次再 poll_read 时会再次得到 0 字节（EOF 信号）。
                    // 不能 buf.clear()，否则会丢弃本轮已填入的数据。
                    return Poll::Ready(Ok(()));
                }
                Poll::Pending => {
                    if is_read_data_tag {
                        return Poll::Ready(Ok(()));
                    }
                    return Poll::Pending;
                }
            }
        }
    }
}

impl<S> AsyncWrite for WebSocketAsyncIo<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        // 确保sink就绪
        match futures::ready!(Pin::new(&mut self.ws_stream).poll_ready(cx)) {
            Ok(()) => {
                let message = Message::Binary(Bytes::copy_from_slice(buf));
                if let Err(e) = Pin::new(&mut self.ws_stream).start_send(message) {
                    return Poll::Ready(Err(io::Error::other(e)));
                }
                Poll::Ready(Ok(buf.len()))
            }
            Err(e) => Poll::Ready(Err(io::Error::other(e))),
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.ws_stream)
            .poll_flush(cx)
            .map_err(|e| io::Error::other(e.to_string()))
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.ws_stream)
            .poll_close(cx)
            .map_err(|e| io::Error::other(e.to_string()))
    }
}
