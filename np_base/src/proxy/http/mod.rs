use crate::net::{SendMessageFuncType, WriterMessage};
use crate::proxy::inlet::InletProxyType;
use crate::proxy::proxy_context::{ProxyContext, ProxyContextData};
use crate::proxy::ProxyMessage;
use anyhow::anyhow;
use async_trait::async_trait;
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use bytes::Bytes;
use log::error;
use std::collections::HashSet;
use std::fmt::Write as FmtWrite;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;

enum Status {
    Free,
    Connecting,
    Running,
    Invalid,
}

pub struct HttpContext {
    status: Status,
    cache_data: Vec<u8>,
    write_to_peer_tx: Option<UnboundedSender<WriterMessage>>,
    peer_addr: Option<SocketAddr>,
    ctx_data: Option<Arc<ProxyContextData>>,
    is_connect_method: bool,
}

const PROXY_AUTH_HEADER: &str = "Proxy-Authorization";
const PROXY_AUTH_REQUIRED_RESPONSE: &[u8] = b"HTTP/1.1 407 Proxy Authentication Required\r\nProxy-Authenticate: Basic realm=\"Proxy\"\r\n\r\n";
const BAD_GATEWAY_HEADER: &[u8] =
    b"HTTP/1.1 502 Bad Gateway\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n";
#[async_trait]
impl ProxyContext for HttpContext {
    async fn on_start(
        &mut self,
        ctx_data: Arc<ProxyContextData>,
        peer_addr: SocketAddr,
        write_to_peer_tx: UnboundedSender<WriterMessage>,
    ) -> anyhow::Result<()> {
        self.write_to_peer_tx = Some(write_to_peer_tx);
        self.peer_addr = Some(peer_addr);
        self.ctx_data = Some(ctx_data);
        Ok(())
    }

    async fn on_recv_peer_data(
        &mut self,
        ctx_data: Arc<ProxyContextData>,
        data: Bytes,
    ) -> anyhow::Result<()> {
        match self.status {
            Status::Free => {
                self.cache_data.extend_from_slice(&data);

                let mut headers = [httparse::EMPTY_HEADER; 64];
                let mut req = httparse::Request::new(&mut headers);
                let res: httparse::Status<usize> = req.parse(&self.cache_data)?;

                match res {
                    httparse::Status::Partial => {
                        // 继续接收数据
                        return Ok(());
                    }
                    httparse::Status::Complete(head_size) => {
                        if !http_authorization(&ctx_data, &req)? {
                            self.status = Status::Invalid;
                            self.send_and_close(PROXY_AUTH_REQUIRED_RESPONSE.to_vec())
                                .await?;
                            return Ok(());
                        }

                        // let method = req.method.ok_or_else(|| anyhow!("parse http method error"))?.to_string();
                        let mut path = req
                            .path
                            .ok_or_else(|| anyhow!("parse http path error"))?
                            .to_string();

                        // 如果输入没有协议，先添加一个假的协议以便解析
                        if !path.starts_with("http://") && !path.starts_with("https://") {
                            path = format!("http://{}", path);
                        }

                        let url =
                            url::Url::parse(&path).map_err(|_| anyhow!("parse http path error"))?;

                        if req.method == Some("CONNECT") {
                            self.is_connect_method = true;
                            let response = format!(
                                "HTTP/1.1 200 Connection Established\r\nProxy-Agent: npipe/{}\r\n\r\n",
                                format_httparse_request_version(req.version)
                            );
                            self.cache_data = response.into_bytes();
                        } else {
                            // 修改header
                            let proxy_headers: HashSet<&'static [u8]> = HashSet::from([
                                // b"proxy-connection".as_slice(),
                                // b"proxy-authenticate".as_slice(),
                                // b"proxy-authorization".as_slice(),
                                b"forwarded".as_slice(),
                                b"x-forwarded-for".as_slice(),
                                b"x-forwarded-host".as_slice(),
                                b"x-forwarded-proto".as_slice(),
                                b"via".as_slice(),
                                // b"connection".as_slice(), // 有时也需要移除
                            ]);

                            let mut headers_vec: Vec<_> = req.headers.to_vec();
                            headers_vec.retain(|header| {
                                let binding = header.name.to_ascii_lowercase();
                                let name = binding.as_bytes();
                                !proxy_headers.contains(name) && !name.starts_with(b"proxy-")
                            });
                            req.headers = &mut *headers_vec;

                            let mut new_data = format_httparse_request(&req).into_bytes();
                            new_data.extend_from_slice(&self.cache_data[head_size..]);
                            self.cache_data = new_data;
                        }

                        let host = url
                            .host_str()
                            .ok_or_else(|| anyhow!("parse http host error"))?;

                        let port = url
                            .port_or_known_default()
                            .ok_or_else(|| anyhow!("parse http port error"))?;

                        self.status = Status::Connecting;
                        // println!("connect: {}:{}", host, port);

                        // 发送连接请求
                        ctx_data
                            .output
                            .send(ProxyMessage::I2oConnect(
                                ctx_data.get_session_id(),
                                InletProxyType::HTTP.to_u8(),
                                true,
                                ctx_data.common_data.is_compressed,
                                format!("{}:{}", host, port),
                                ctx_data.common_data.encryption_method.to_string(),
                                BASE64_STANDARD
                                    .encode(ctx_data.common_data.encryption_key.as_slice()),
                                self.peer_addr.as_ref().unwrap().to_string(),
                            ))
                            .await?;
                    }
                }
            }
            Status::Invalid => {}
            Status::Connecting => {}
            Status::Running => {
                let encoded = ctx_data
                    .common_data
                    .encode_data_and_limiting(data) // data: Bytes，无需 to_vec()
                    .await?;
                ctx_data
                    .output
                    .send(ProxyMessage::I2oSendData(
                        ctx_data.get_session_id(),
                        encoded,
                    ))
                    .await?;
            }
        }

        Ok(())
    }

    async fn on_recv_proxy_message(&mut self, proxy_message: ProxyMessage) -> anyhow::Result<()> {
        match proxy_message {
            ProxyMessage::O2iConnect(_session_id, success, error_msg) => {
                if success {
                    self.status = Status::Running;
                    if self.is_connect_method {
                        // mem::take: O(1) 转移所有权，不拷贝数据
                        self.write_to_peer_tx
                            .as_ref()
                            .unwrap()
                            .send(WriterMessage::Send(
                                Bytes::from(std::mem::take(&mut self.cache_data)),
                                true,
                            ))?;
                    } else {
                        let data = self
                            .ctx_data
                            .as_ref()
                            .unwrap()
                            .common_data
                            // mem::take 是 O(1)，Bytes::from(Vec) 也是 O(1)，合计零拷贝
                            .encode_data_and_limiting(Bytes::from(std::mem::take(
                                &mut self.cache_data,
                            )))
                            .await?;
                        self.ctx_data
                            .as_ref()
                            .unwrap()
                            .output
                            .send(ProxyMessage::I2oSendData(
                                self.ctx_data.as_ref().unwrap().get_session_id(),
                                data,
                            ))
                            .await?;
                    }
                    // mem::take 已经将 cache_data 置为空 Vec，无需再 clear/shrink
                } else {
                    error!("http proxy connect error: {error_msg}");
                    self.status = Status::Invalid;

                    let mut response = BAD_GATEWAY_HEADER.to_vec();
                    response.extend_from_slice(
                        format!(
                            "<html><body>\
                            <h1>502 Bad Gateway</h1>\
                            <p>Proxy connection failed: {}</p>\
                            </body></html>",
                            error_msg
                        )
                        .as_bytes(),
                    );
                    self.send_and_close(response).await?;
                }
            }
            ProxyMessage::O2iRecvData(session_id, data) => {
                let data_len = data.len();
                let decoded = self
                    .ctx_data
                    .as_ref()
                    .unwrap()
                    .common_data
                    .decode_data(data)?;

                // 写入完毕回调
                let output = self.ctx_data.as_ref().unwrap().output.clone();
                let callback: SendMessageFuncType = Box::new(move || {
                    let output = output.clone();
                    Box::pin(async move {
                        let _ = output
                            .send(ProxyMessage::I2oRecvDataResult(session_id, data_len))
                            .await;
                    })
                });

                self.write_to_peer_tx
                    .as_ref()
                    .unwrap()
                    .send(WriterMessage::SendAndThen(decoded, callback))?;
            }
            ProxyMessage::O2iDisconnect(_) => {
                self.write_to_peer_tx
                    .as_ref()
                    .unwrap()
                    .send(WriterMessage::Close)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn is_ready_for_read(&self) -> bool {
        !matches!(self.status, Status::Connecting | Status::Invalid)
    }
}

impl HttpContext {
    pub fn new() -> Self {
        Self {
            status: Status::Free,
            cache_data: Vec::new(),
            write_to_peer_tx: None,
            peer_addr: None,
            ctx_data: None,
            is_connect_method: false,
        }
    }

    async fn send_and_close(&self, data: Vec<u8>) -> anyhow::Result<()> {
        self.write_to_peer_tx
            .as_ref()
            .unwrap()
            .send(WriterMessage::Send(Bytes::from(data), true))?;
        self.write_to_peer_tx
            .as_ref()
            .unwrap()
            .send(WriterMessage::CloseDelayed(Duration::from_secs(1)))?;
        Ok(())
    }
}

fn http_authorization(
    ctx_data: &Arc<ProxyContextData>,
    request: &httparse::Request,
) -> anyhow::Result<bool> {
    let username = &ctx_data.data_ex.username;
    let password = &ctx_data.data_ex.password;

    if username.is_empty() || password.is_empty() {
        return Ok(true);
    }

    if let Some(header) = request.headers.iter().find(|h| h.name == PROXY_AUTH_HEADER) {
        if let Some(credential) = header.value.strip_prefix(b"Basic ") {
            if let Ok(decoded) = BASE64_STANDARD.decode(credential) {
                let decoded_str = std::str::from_utf8(&decoded).unwrap_or("");
                let parts: Vec<&str> = decoded_str.split(':').collect();
                if parts.len() == 2 && parts[0] == username && parts[1] == password {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}

fn format_httparse_request_version(version: Option<u8>) -> &'static str {
    match version {
        Some(0) => "HTTP/1.0",
        Some(1) => "HTTP/1.1",
        Some(2) => "HTTP/2", // 注意：HTTP/2 通常不适用文本格式化
        Some(3) => "HTTP/3", // 注意：HTTP/3 使用 QUIC，非文本协议
        _ => "HTTP/1.1",     // 默认
    }
}

fn format_httparse_request(req: &httparse::Request) -> String {
    let version = format_httparse_request_version(req.version);

    // 预估容量：请求行约 64B + 每个头约 40B，减少 realloc 次数
    let estimated = 64 + req.headers.len() * 40 + 2;
    let mut result = String::with_capacity(estimated);

    // 请求行直接写入，避免中间 String 分配
    let _ = write!(
        result,
        "{} {} {}\r\n",
        req.method.unwrap_or("GET"),
        req.path.unwrap_or("/"),
        version
    );

    // 每个头用 write! 直接追加，消除原来 &format!(...) 的 N 次临时堆分配
    for h in req.headers.iter() {
        let name = std::str::from_utf8(h.name.as_bytes()).unwrap_or("");
        let value = std::str::from_utf8(h.value).unwrap_or("");
        let _ = write!(result, "{}: {}\r\n", name, value);
    }
    result.push_str("\r\n");
    result
}
