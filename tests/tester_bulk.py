"""
大数据量传输测试
================
通过隧道连续发送大量数据，验证：
  1. 数据完整性（SHA-256 哈希比对）
  2. 无丢包、无乱序
  3. 多轮往返均正常
  4. 吞吐量统计

支持的底层连接方式：
  - TCP 直连（用于 TCP 隧道测试）
  - SOCKS5 连接（用于 SOCKS5 隧道测试）
  - HTTP CONNECT 连接（用于 HTTP 代理隧道测试）
"""
import hashlib
import logging
import os
import socket
import struct
import time
import base64

logger = logging.getLogger("tester.bulk")


# ---------------------------------------------------------------------------
# 可调参数
# ---------------------------------------------------------------------------
DEFAULT_CHUNK_SIZE  = 64 * 1024          # 每次 send 的块大小：64 KB
DEFAULT_TOTAL_SIZE  = 4 * 1024 * 1024    # 默认单轮数据总量：4 MB
DEFAULT_ROUNDS      = 3                  # 轮次
DEFAULT_TIMEOUT     = 60                 # 每次操作超时（秒）


# ---------------------------------------------------------------------------
# 内部工具
# ---------------------------------------------------------------------------

def _recv_exact(sock: socket.socket, n: int, timeout_deadline: float) -> bytes:
    buf = b""
    while len(buf) < n:
        remaining = timeout_deadline - time.time()
        if remaining <= 0:
            raise TimeoutError(f"读取超时（期望 {n} 字节，已收 {len(buf)} 字节）")
        sock.settimeout(min(remaining, 5.0))
        chunk = sock.recv(min(65536, n - len(buf)))
        if not chunk:
            raise ConnectionError(f"连接意外关闭（期望 {n} 字节，已收 {len(buf)} 字节）")
        buf += chunk
    return buf


def _send_recv_round(
    sock: socket.socket,
    total_size: int,
    chunk_size: int,
    timeout: float,
) -> tuple[float, str]:
    """
    一轮：先发送 total_size 字节随机数据（分块发送），再接收相同数量字节，
    比对 SHA-256。

    协议格式（在隧道之上使用简单的长度前缀帧）：
      [4 bytes big-endian length][N bytes payload]

    返回 (throughput_mbps, error_msg)。error_msg 为空表示成功。
    """
    payload = os.urandom(total_size)
    send_hash = hashlib.sha256(payload).hexdigest()

    # --- 发送：length-prefix + payload ---
    header = struct.pack(">I", total_size)
    t0 = time.time()
    deadline = t0 + timeout

    sock.settimeout(min(timeout, 10.0))
    try:
        sock.sendall(header)
        sent = 0
        while sent < total_size:
            end = min(sent + chunk_size, total_size)
            sock.sendall(payload[sent:end])
            sent = end
    except Exception as e:
        return 0.0, f"发送失败：{e}"

    # --- 接收：先读 4 字节长度，再读 payload ---
    try:
        resp_header = _recv_exact(sock, 4, deadline)
        resp_len = struct.unpack(">I", resp_header)[0]
        if resp_len != total_size:
            return 0.0, f"回显长度不匹配：期望 {total_size}，收到 {resp_len}"
        received = _recv_exact(sock, resp_len, deadline)
    except Exception as e:
        return 0.0, f"接收失败：{e}"

    elapsed = time.time() - t0

    # --- 完整性校验 ---
    recv_hash = hashlib.sha256(received).hexdigest()
    if recv_hash != send_hash:
        return 0.0, f"数据完整性校验失败（SHA-256 不一致）"

    # --- 吞吐量（双向合计 / 单向，取单向发送量 / 时间）---
    mbps = (total_size / elapsed) / (1024 * 1024)
    return mbps, ""


# ---------------------------------------------------------------------------
# Echo server 适配：让普通 echo server 支持长度前缀帧协议
# ---------------------------------------------------------------------------

from echo_server import TCPEchoServer as _RawTCPEchoServer
import threading


class FramedEchoServer:
    """
    带长度前缀帧的 TCP 回显服务器，供大数据量测试使用。
    协议：[4 bytes big-endian length][N bytes payload] → 原样回显
    """

    def __init__(self, host: str, port: int):
        self.host = host
        self.port = port
        self._server_socket: socket.socket | None = None
        self._thread: threading.Thread | None = None
        self._stop_event = threading.Event()

    def start(self):
        self._server_socket = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self._server_socket.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        self._server_socket.bind((self.host, self.port))
        self._server_socket.listen(10)
        self._server_socket.settimeout(1.0)
        self._stop_event.clear()
        self._thread = threading.Thread(target=self._serve, daemon=True)
        self._thread.start()
        logger.info(f"FramedEchoServer started on {self.host}:{self.port}")

    def stop(self):
        self._stop_event.set()
        if self._server_socket:
            try:
                self._server_socket.close()
            except Exception:
                pass
        if self._thread:
            self._thread.join(timeout=3)

    def _serve(self):
        while not self._stop_event.is_set():
            try:
                conn, addr = self._server_socket.accept()
                t = threading.Thread(target=self._handle, args=(conn,), daemon=True)
                t.start()
            except socket.timeout:
                continue
            except OSError:
                break

    def _handle(self, conn: socket.socket):
        try:
            conn.settimeout(120)
            while True:
                # 读取 4 字节长度头
                header = b""
                while len(header) < 4:
                    chunk = conn.recv(4 - len(header))
                    if not chunk:
                        return
                    header += chunk
                n = struct.unpack(">I", header)[0]

                # 读取 payload
                payload = b""
                while len(payload) < n:
                    chunk = conn.recv(min(65536, n - len(payload)))
                    if not chunk:
                        return
                    payload += chunk

                # 回显（带长度前缀）
                conn.sendall(header + payload)
        except Exception as e:
            logger.debug(f"FramedEchoServer client error: {e}")
        finally:
            conn.close()


# ---------------------------------------------------------------------------
# 连接工厂
# ---------------------------------------------------------------------------

def _connect_direct(host: str, port: int, timeout: float) -> socket.socket:
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.settimeout(timeout)
    sock.connect((host, port))
    return sock


def _connect_socks5(
    proxy_host: str, proxy_port: int,
    target_host: str, target_port: int,
    username: str = "", password: str = "",
    timeout: float = 30,
) -> socket.socket:
    """复用 tester_socks5 中的连接逻辑"""
    from tester_socks5 import _socks5_connect
    return _socks5_connect(proxy_host, proxy_port, target_host, target_port,
                           username=username, password=password, timeout=timeout)


def _connect_http_connect(
    proxy_host: str, proxy_port: int,
    target_host: str, target_port: int,
    username: str = "", password: str = "",
    timeout: float = 30,
) -> socket.socket:
    """复用 tester_http 中的 CONNECT 逻辑"""
    from tester_http import _do_connect, HttpProxyError
    sock, status_code, status_text = _do_connect(
        proxy_host, proxy_port, target_host, target_port,
        username=username, password=password, timeout=timeout,
    )
    if status_code != 200:
        sock.close()
        raise HttpProxyError(f"HTTP CONNECT 失败：{status_code} {status_text}")
    return sock


# ---------------------------------------------------------------------------
# 公开测试函数
# ---------------------------------------------------------------------------

def _run_bulk(
    connect_fn,
    label: str,
    total_size: int,
    chunk_size: int,
    rounds: int,
    timeout: float,
) -> tuple[bool, str]:
    """通用批量传输执行器"""
    total_mb = total_size / (1024 * 1024)
    all_mbps = []
    errors = []

    for r in range(1, rounds + 1):
        sock = None
        try:
            sock = connect_fn()
            mbps, err = _send_recv_round(sock, total_size, chunk_size, timeout)
            if err:
                errors.append(f"第{r}轮：{err}")
                logger.warning(f"[{label}] 第{r}轮失败：{err}")
            else:
                all_mbps.append(mbps)
                logger.info(f"[{label}] 第{r}轮：{total_mb:.1f} MB  {mbps:.2f} MB/s")
        except Exception as e:
            errors.append(f"第{r}轮连接失败：{e}")
        finally:
            if sock:
                try:
                    sock.close()
                except Exception:
                    pass

    passed = len(all_mbps)
    total_rounds = rounds
    if errors:
        err_summary = "；".join(errors)
        if passed == 0:
            return False, f"全部 {total_rounds} 轮失败：{err_summary}"
        return False, f"{passed}/{total_rounds} 轮通过，失败原因：{err_summary}"

    avg_mbps = sum(all_mbps) / len(all_mbps)
    min_mbps = min(all_mbps)
    return (
        True,
        f"{total_rounds} 轮全部通过 ✓  {total_mb:.1f}MB×{total_rounds}轮  "
        f"平均 {avg_mbps:.2f} MB/s  最低 {min_mbps:.2f} MB/s",
    )


def test_bulk_tcp(
    inlet_host: str,
    inlet_port: int,
    total_size: int = DEFAULT_TOTAL_SIZE,
    chunk_size: int = DEFAULT_CHUNK_SIZE,
    rounds: int = DEFAULT_ROUNDS,
    timeout: float = DEFAULT_TIMEOUT,
) -> tuple[bool, str]:
    """TCP 隧道大数据量传输测试"""
    def connect():
        return _connect_direct(inlet_host, inlet_port, timeout=30)
    return _run_bulk(connect, "TCP-bulk", total_size, chunk_size, rounds, timeout)


def test_bulk_socks5(
    proxy_host: str,
    proxy_port: int,
    target_host: str,
    target_port: int,
    username: str = "",
    password: str = "",
    total_size: int = DEFAULT_TOTAL_SIZE,
    chunk_size: int = DEFAULT_CHUNK_SIZE,
    rounds: int = DEFAULT_ROUNDS,
    timeout: float = DEFAULT_TIMEOUT,
) -> tuple[bool, str]:
    """SOCKS5 代理大数据量传输测试"""
    def connect():
        return _connect_socks5(proxy_host, proxy_port, target_host, target_port,
                               username=username, password=password, timeout=30)
    return _run_bulk(connect, "SOCKS5-bulk", total_size, chunk_size, rounds, timeout)


def test_bulk_http(
    proxy_host: str,
    proxy_port: int,
    target_host: str,
    target_port: int,
    username: str = "",
    password: str = "",
    total_size: int = DEFAULT_TOTAL_SIZE,
    chunk_size: int = DEFAULT_CHUNK_SIZE,
    rounds: int = DEFAULT_ROUNDS,
    timeout: float = DEFAULT_TIMEOUT,
) -> tuple[bool, str]:
    """HTTP CONNECT 代理大数据量传输测试"""
    def connect():
        return _connect_http_connect(proxy_host, proxy_port, target_host, target_port,
                                     username=username, password=password, timeout=30)
    return _run_bulk(connect, "HTTP-bulk", total_size, chunk_size, rounds, timeout)

