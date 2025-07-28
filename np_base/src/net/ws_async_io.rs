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
    read_buf: Vec<u8>, // 用于缓存未读完的数据
}

impl<S> WebSocketAsyncIo<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    pub fn new(ws_stream: WebSocketStream<S>) -> Self {
        Self {
            ws_stream,
            read_buf: Vec::new(),
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
        // 如果 read_buf 有剩余数据，先返回
        if !self.read_buf.is_empty() {
            let n = std::cmp::min(self.read_buf.len(), buf.remaining());
            buf.put_slice(&self.read_buf[..n]);
            self.read_buf.drain(..n);
            return Poll::Ready(Ok(()));
        }

        // 使用 `futures::Stream::poll_next` 读取 WebSocket 消息
        match Pin::new(&mut self.ws_stream).poll_next(cx) {
            Poll::Ready(Some(Ok(msg))) => {
                match msg {
                    Message::Text(text) => {
                        self.read_buf.extend(text.as_bytes());
                    }
                    Message::Binary(data) => {
                        self.read_buf.extend(data);
                    }
                    Message::Ping(payload) => {
                        // 自动回复Pong
                        // 确保Sink就绪后再发送Pong
                        match Pin::new(&mut self.ws_stream).poll_ready(cx) {
                            Poll::Ready(Ok(())) => {
                                if let Err(e) =
                                    Pin::new(&mut self.ws_stream).start_send(Message::Pong(payload))
                                {
                                    return Poll::Ready(Err(io::Error::new(
                                        io::ErrorKind::Other,
                                        e,
                                    )));
                                }
                            }
                            Poll::Ready(Err(e)) => {
                                return Poll::Ready(Err(io::Error::new(io::ErrorKind::Other, e)));
                            }
                            Poll::Pending => {
                                // Sink未就绪，返回Pending
                                return Poll::Pending;
                            }
                        }
                    }
                    Message::Pong(_) => {
                        // 丢弃 Pong 消息，继续读取
                        // return self.poll_read(cx, buf);
                    }
                    Message::Close(_) => {
                        return Poll::Ready(Err(io::Error::new(
                            io::ErrorKind::ConnectionAborted,
                            "WebSocket closed",
                        )));
                    }
                    Message::Frame(_) => {
                        // 处理 Frame 消息，这里简单处理为忽略
                        // return self.poll_read(cx, buf);
                    }
                }

                // 再次尝试填充 buf
                if !self.read_buf.is_empty() {
                    let n = std::cmp::min(self.read_buf.len(), buf.remaining());
                    buf.put_slice(&self.read_buf[..n]);
                    self.read_buf.drain(..n);
                    Poll::Ready(Ok(()))
                } else {
                    // 理论上不会走到这里，因为至少有一个消息被处理
                    Poll::Ready(Ok(()))
                }
            }
            Poll::Ready(Some(Err(e))) => {
                Poll::Ready(Err(io::Error::new(io::ErrorKind::Other, e.to_string())))
            }
            Poll::Ready(None) => Poll::Ready(Ok(())), // EOF
            Poll::Pending => Poll::Pending,
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
        match Pin::new(&mut self.ws_stream).poll_ready(cx) {
            Poll::Ready(Ok(())) => {
                let message = Message::Binary(Bytes::copy_from_slice(buf));
                if let Err(e) = Pin::new(&mut self.ws_stream).start_send(message) {
                    return Poll::Ready(Err(io::Error::new(io::ErrorKind::Other, e)));
                }
                Poll::Ready(Ok(buf.len()))
            }
            Poll::Ready(Err(e)) => Poll::Ready(Err(io::Error::new(io::ErrorKind::Other, e))),
            Poll::Pending => Poll::Pending,
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.ws_stream)
            .poll_flush(cx)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.ws_stream)
            .poll_close(cx)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
    }
}
