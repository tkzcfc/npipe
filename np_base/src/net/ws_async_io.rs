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

    /// 处理数据消息：优先直接写入 buf，剩余部分缓存
    fn process_data(&mut self, data: &[u8], buf: &mut ReadBuf<'_>) {
        let remaining = buf.remaining();

        if data.len() <= remaining {
            // 整个消息可直接写入
            buf.put_slice(&data);
        } else {
            // 拆分消息：部分写入，剩余缓存
            buf.put_slice(&data[..remaining]);
            self.read_buf = data[remaining..].to_vec();
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
            // 优先从缓存读取残留数据
            if !self.read_buf.is_empty() {
                is_read_data_tag = true;
                let n = std::cmp::min(self.read_buf.len(), buf.remaining());
                buf.put_slice(&self.read_buf[..n]);
                self.read_buf.drain(..n);
            }
            if buf.remaining() == 0 {
                return Poll::Ready(Ok(())); // 缓冲区已满
            }

            // 使用 `futures::Stream::poll_next` 读取 WebSocket 消息
            match Pin::new(&mut self.ws_stream).poll_next(cx) {
                Poll::Ready(Some(Ok(msg))) => {
                    match msg {
                        Message::Text(text) => {
                            self.process_data(text.as_bytes(), buf);
                            is_read_data_tag = true;
                        }
                        Message::Binary(data) => {
                            self.process_data(&data, buf);
                            is_read_data_tag = true;
                        }
                        Message::Ping(payload) => {
                            // 自动回复Pong,确保Sink就绪后再发送Pong
                            match Pin::new(&mut self.ws_stream).poll_ready(cx) {
                                Poll::Ready(Ok(())) => {
                                    if let Err(e) = Pin::new(&mut self.ws_stream)
                                        .start_send(Message::Pong(payload))
                                    {
                                        return Poll::Ready(Err(io::Error::new(
                                            io::ErrorKind::Other,
                                            e,
                                        )));
                                    }
                                }
                                Poll::Ready(Err(e)) => {
                                    return Poll::Ready(Err(io::Error::new(
                                        io::ErrorKind::Other,
                                        e,
                                    )));
                                }
                                Poll::Pending => {
                                    // 如果没有发送就算忽略本次Ping了,也可以将本次Ping请求缓存下来,那样太复杂了
                                    // Sink未就绪，返回Pending
                                    // return Poll::Pending;
                                }
                            }
                        }
                        Message::Close(_) => {
                            return Poll::Ready(Err(io::Error::new(
                                io::ErrorKind::ConnectionAborted,
                                "WebSocket closed",
                            )));
                        }
                        Message::Frame(_) | Message::Pong(_) => {
                            // 丢弃 Pong 消息 和 Frame 消息
                        }
                    }
                }
                Poll::Ready(Some(Err(e))) => {
                    return Poll::Ready(Err(io::Error::new(io::ErrorKind::Other, e.to_string())));
                }
                Poll::Ready(None) => {
                    buf.clear();
                    return Poll::Ready(Ok(()));
                } // EOF
                Poll::Pending => {
                    if is_read_data_tag {
                        return Poll::Ready(Ok(()));
                    }
                    // 如果没有数据可读，返回 Pending
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
                    return Poll::Ready(Err(io::Error::new(io::ErrorKind::Other, e)));
                }
                Poll::Ready(Ok(buf.len()))
            }
            Err(e) => Poll::Ready(Err(io::Error::new(io::ErrorKind::Other, e))),
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
