"""
HTTP 代理隧道测试

支持两种模式：
  1. CONNECT 隧道（用于 HTTPS 或任意 TCP）
     客户端发送 CONNECT host:port HTTP/1.1，代理建立隧道后双方直通。
  2. 普通 HTTP 代理请求（直接转发 GET/POST 等）

认证：npipe HTTP 代理使用 Proxy-Authorization: Basic base64(user:pass)
"""
import socket
import base64
import logging
import os
import time

logger = logging.getLogger("tester.http")

TEST_DATA = b"Hello npipe HTTP-CONNECT tunnel! " + os.urandom(32)


class HttpProxyError(Exception):
    pass


def _recv_until(sock: socket.socket, delimiter: bytes, max_bytes: int = 8192) -> bytes:
    buf = b""
    while delimiter not in buf:
        if len(buf) >= max_bytes:
            raise HttpProxyError("HTTP响应头超过最大长度")
        chunk = sock.recv(256)
        if not chunk:
            raise HttpProxyError("连接意外关闭（读取响应头时）")
        buf += chunk
    return buf


def _build_proxy_auth_header(username: str, password: str) -> str:
    """生成 Proxy-Authorization: Basic ... 头部行（含末尾 \\r\\n）"""
    credential = base64.b64encode(f"{username}:{password}".encode()).decode()
    return f"Proxy-Authorization: Basic {credential}\r\n"


def _do_connect(
    proxy_host: str,
    proxy_port: int,
    target_host: str,
    target_port: int,
    username: str = "",
    password: str = "",
    timeout: float = 10,
) -> tuple[socket.socket, int, str]:
    """
    建立到代理的 TCP 连接并发送 CONNECT 请求。
    返回 (sock, status_code, status_text)。
    状态码 200 表示隧道建立成功。
    """
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.settimeout(timeout)
    sock.connect((proxy_host, proxy_port))

    auth_line = _build_proxy_auth_header(username, password) if (username and password) else ""
    connect_req = (
        f"CONNECT {target_host}:{target_port} HTTP/1.1\r\n"
        f"Host: {target_host}:{target_port}\r\n"
        f"User-Agent: npipe-tester/1.0\r\n"
        f"{auth_line}"
        f"\r\n"
    ).encode()
    sock.sendall(connect_req)

    response = _recv_until(sock, b"\r\n\r\n")
    header_text = response.split(b"\r\n\r\n")[0].decode(errors="replace")
    first_line = header_text.split("\r\n")[0]
    parts = first_line.split(" ", 2)
    if len(parts) < 2:
        sock.close()
        raise HttpProxyError(f"HTTP代理响应无效：{first_line!r}")
    status_code = int(parts[1])
    status_text = parts[2] if len(parts) > 2 else ""
    return sock, status_code, status_text


# ---------------------------------------------------------------------------
# 公开测试函数
# ---------------------------------------------------------------------------

def test_http_connect_tunnel(
    proxy_host: str,
    proxy_port: int,
    target_host: str,
    target_port: int,
    username: str = "",
    password: str = "",
    timeout: float = 10,
) -> tuple[bool, str]:
    """
    通过 HTTP CONNECT 方法建立隧道，连接到目标 TCP echo server，发送数据并验证回显。
    支持 Proxy-Authorization Basic 认证。

    Returns:
        (success, message)
    """
    sock = None
    try:
        sock, status_code, status_text = _do_connect(
            proxy_host, proxy_port, target_host, target_port,
            username=username, password=password, timeout=timeout,
        )
        logger.debug(f"HTTP CONNECT 响应：{status_code} {status_text}")

        if status_code != 200:
            return False, f"HTTP CONNECT 失败：{status_code} {status_text}"

        # --- 隧道已建立，像普通 TCP 一样通信 ---
        sock.sendall(TEST_DATA)

        received = b""
        deadline = time.time() + timeout
        while len(received) < len(TEST_DATA):
            remaining = deadline - time.time()
            if remaining <= 0:
                return False, f"接收超时：已收 {len(received)}/{len(TEST_DATA)} 字节"
            sock.settimeout(remaining)
            chunk = sock.recv(4096)
            if not chunk:
                break
            received += chunk

        if received == TEST_DATA:
            auth_note = "（含认证）" if username else ""
            return True, f"HTTP CONNECT{auth_note}成功，发送 {len(TEST_DATA)} 字节，回显一致 ✓"
        else:
            return (
                False,
                f"数据不匹配：期望 {len(TEST_DATA)} 字节，实际收到 {len(received)} 字节",
            )
    except ConnectionRefusedError:
        return False, f"连接被拒绝：{proxy_host}:{proxy_port}（HTTP代理入口未监听）"
    except socket.timeout:
        return False, f"连接/接收超时（{timeout}s）"
    except HttpProxyError as e:
        return False, f"HTTP协议错误：{e}"
    except ValueError as e:
        return False, f"HTTP响应解析错误：{e}"
    except Exception as e:
        return False, f"HTTP测试异常：{e}"
    finally:
        if sock:
            try:
                sock.close()
            except Exception:
                pass


def test_http_connect_wrong_auth(
    proxy_host: str,
    proxy_port: int,
    target_host: str,
    target_port: int,
    timeout: float = 10,
) -> tuple[bool, str]:
    """
    使用错误的用户名密码发起 CONNECT 请求，验证代理正确返回 407。

    Returns:
        (success, message)  ← success=True 表示代理确实拒绝了错误凭证
    """
    sock = None
    try:
        sock, status_code, status_text = _do_connect(
            proxy_host, proxy_port, target_host, target_port,
            username="wrong_user", password="wrong_pass", timeout=timeout,
        )
        if status_code == 407:
            return True, f"代理正确拒绝了错误凭证（407 {status_text}）✓"
        elif status_code == 200:
            return False, "代理接受了错误凭证（预期被拒绝）✗"
        else:
            return False, f"收到意外状态码：{status_code} {status_text}"
    except ConnectionRefusedError:
        return False, f"连接被拒绝：{proxy_host}:{proxy_port}"
    except socket.timeout:
        return False, f"超时（{timeout}s）"
    except HttpProxyError as e:
        # 有些代理在认证失败时直接断开连接，也视为"正确拒绝"
        return True, f"代理关闭了连接（认证失败行为符合预期）：{e} ✓"
    except Exception as e:
        return False, f"HTTP错误认证测试异常：{e}"
    finally:
        if sock:
            try:
                sock.close()
            except Exception:
                pass


def test_http_proxy_request(
    proxy_host: str,
    proxy_port: int,
    target_url: str,
    username: str = "",
    password: str = "",
    timeout: float = 10,
) -> tuple[bool, str]:
    """
    通过 HTTP 代理发送 GET 请求（普通 HTTP 代理模式，非 CONNECT）。
    """
    try:
        import urllib.parse
        parsed = urllib.parse.urlparse(target_url)
        host = parsed.hostname
        port = parsed.port or 80

        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.settimeout(timeout)
        sock.connect((proxy_host, proxy_port))

        auth_line = _build_proxy_auth_header(username, password) if (username and password) else ""
        req = (
            f"GET {target_url} HTTP/1.1\r\n"
            f"Host: {host}:{port}\r\n"
            f"User-Agent: npipe-tester/1.0\r\n"
            f"Connection: close\r\n"
            f"{auth_line}"
            f"\r\n"
        ).encode()
        sock.sendall(req)

        response = _recv_until(sock, b"\r\n\r\n")
        header_text = response.split(b"\r\n\r\n")[0].decode(errors="replace")
        first_line = header_text.split("\r\n")[0]
        parts = first_line.split(" ", 2)
        sock.close()

        if len(parts) < 2:
            return False, f"HTTP代理响应无效：{first_line!r}"
        status_code = int(parts[1])
        if 200 <= status_code < 400:
            return True, f"HTTP代理请求成功：{status_code} ✓"
        else:
            reason = parts[2] if len(parts) > 2 else ""
            return False, f"HTTP代理请求失败：{status_code} {reason}"
    except ConnectionRefusedError:
        return False, f"连接被拒绝：{proxy_host}:{proxy_port}"
    except socket.timeout:
        return False, f"连接/接收超时（{timeout}s）"
    except HttpProxyError as e:
        return False, f"HTTP协议错误：{e}"
    except Exception as e:
        return False, f"HTTP测试异常：{e}"
