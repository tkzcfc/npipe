"""
SOCKS5 代理隧道测试

SOCKS5 协议握手流程（RFC 1928）：
  1. 客户端 -> 代理：握手（支持的认证方法列表）
  2. 代理 -> 客户端：选择的认证方法
  3. （若有认证）用户名密码认证
  4. 客户端 -> 代理：CONNECT 请求（目标地址+端口）
  5. 代理 -> 客户端：CONNECT 响应
  6. 双方通过代理直接通信
"""
import socket
import struct
import logging
import os
import time

logger = logging.getLogger("tester.socks5")

TEST_DATA = b"Hello npipe SOCKS5 tunnel! " + os.urandom(32)

SOCKS5_VERSION = 0x05
AUTH_NO_AUTH = 0x00
AUTH_USERNAME_PASSWORD = 0x02
AUTH_NO_ACCEPTABLE = 0xFF
CMD_CONNECT = 0x01
ATYP_IPV4 = 0x01
ATYP_DOMAIN = 0x03
ATYP_IPV6 = 0x04
REP_SUCCESS = 0x00


class Socks5Error(Exception):
    pass


def _socks5_connect(
    proxy_host: str,
    proxy_port: int,
    target_host: str,
    target_port: int,
    username: str = "",
    password: str = "",
    timeout: float = 10,
) -> socket.socket:
    """
    建立 SOCKS5 连接并完成协议握手，返回已连接好的 socket。
    """
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.settimeout(timeout)
    sock.connect((proxy_host, proxy_port))

    # --- 1. 握手：告知支持的认证方法 ---
    if username and password:
        methods = bytes([AUTH_NO_AUTH, AUTH_USERNAME_PASSWORD])
    else:
        methods = bytes([AUTH_NO_AUTH])

    sock.sendall(bytes([SOCKS5_VERSION, len(methods)]) + methods)

    # --- 2. 收到代理选择的认证方法 ---
    resp = _recv_exact(sock, 2)
    if resp[0] != SOCKS5_VERSION:
        raise Socks5Error(f"SOCKS5握手：版本不匹配 {resp[0]}")
    chosen_method = resp[1]

    if chosen_method == AUTH_NO_ACCEPTABLE:
        raise Socks5Error("SOCKS5握手：代理无可接受的认证方法")

    # --- 3. 认证 ---
    if chosen_method == AUTH_USERNAME_PASSWORD:
        if not username or not password:
            raise Socks5Error("SOCKS5：代理要求用户名密码但未提供")
        uname = username.encode()
        passwd = password.encode()
        auth_req = bytes([0x01, len(uname)]) + uname + bytes([len(passwd)]) + passwd
        sock.sendall(auth_req)
        auth_resp = _recv_exact(sock, 2)
        if auth_resp[1] != 0x00:
            raise Socks5Error(f"SOCKS5：认证失败（代码 {auth_resp[1]}）")
    elif chosen_method == AUTH_NO_AUTH:
        pass  # 无认证，直接继续
    else:
        raise Socks5Error(f"SOCKS5：不支持的认证方法 {chosen_method}")

    # --- 4. 发出 CONNECT 请求 ---
    # 尝试解析为 IPv4
    try:
        addr_bytes = socket.inet_aton(target_host)
        atyp = ATYP_IPV4
        addr_field = addr_bytes
    except OSError:
        # 使用域名类型
        atyp = ATYP_DOMAIN
        enc = target_host.encode()
        addr_field = bytes([len(enc)]) + enc

    port_bytes = struct.pack(">H", target_port)
    connect_req = bytes([SOCKS5_VERSION, CMD_CONNECT, 0x00, atyp]) + addr_field + port_bytes
    sock.sendall(connect_req)

    # --- 5. 读取 CONNECT 响应 ---
    resp_header = _recv_exact(sock, 4)
    if resp_header[0] != SOCKS5_VERSION:
        raise Socks5Error(f"SOCKS5 CONNECT响应：版本不匹配 {resp_header[0]}")
    rep = resp_header[1]
    if rep != REP_SUCCESS:
        _REPLY_MESSAGES = {
            0x01: "通用失败",
            0x02: "连接被规则拒绝",
            0x03: "网络不可达",
            0x04: "主机不可达",
            0x05: "连接被拒绝",
            0x06: "TTL超时",
            0x07: "不支持的命令",
            0x08: "不支持的地址类型",
        }
        msg = _REPLY_MESSAGES.get(rep, f"未知错误码 {rep:#04x}")
        raise Socks5Error(f"SOCKS5 CONNECT失败：{msg}")

    # 读取剩余的绑定地址字段（BND.ADDR + BND.PORT）
    atyp_resp = resp_header[3]
    if atyp_resp == ATYP_IPV4:
        _recv_exact(sock, 4 + 2)  # 4字节IPv4 + 2字节端口
    elif atyp_resp == ATYP_IPV6:
        _recv_exact(sock, 16 + 2)
    elif atyp_resp == ATYP_DOMAIN:
        domain_len = _recv_exact(sock, 1)[0]
        _recv_exact(sock, domain_len + 2)
    else:
        raise Socks5Error(f"SOCKS5 CONNECT响应：未知地址类型 {atyp_resp}")

    return sock


def _recv_exact(sock: socket.socket, n: int) -> bytes:
    buf = b""
    while len(buf) < n:
        chunk = sock.recv(n - len(buf))
        if not chunk:
            raise Socks5Error(f"SOCKS5：连接意外关闭（期望{n}字节，已收{len(buf)}字节）")
        buf += chunk
    return buf


def test_socks5_wrong_auth(
    proxy_host: str,
    proxy_port: int,
    target_host: str,
    target_port: int,
    timeout: float = 10,
) -> tuple[bool, str]:
    """
    使用错误的用户名密码进行 SOCKS5 认证，验证代理正确拒绝。

    Returns:
        (success, message)  ← success=True 表示代理确实拒绝了错误凭证
    """
    sock = None
    try:
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.settimeout(timeout)
        sock.connect((proxy_host, proxy_port))

        # 握手：声明支持用户名/密码认证
        sock.sendall(bytes([SOCKS5_VERSION, 1, AUTH_USERNAME_PASSWORD]))
        resp = _recv_exact(sock, 2)
        if resp[0] != SOCKS5_VERSION:
            return False, f"握手版本不匹配：{resp[0]}"
        chosen = resp[1]
        if chosen == AUTH_NO_ACCEPTABLE:
            return False, "代理不接受用户名密码认证（隧道配置未启用认证？）"
        if chosen != AUTH_USERNAME_PASSWORD:
            return False, f"代理选择了意外的认证方法：{chosen}"

        # 发送错误凭证
        uname = b"wrong_user"
        passwd = b"wrong_pass"
        auth_req = bytes([0x01, len(uname)]) + uname + bytes([len(passwd)]) + passwd
        sock.sendall(auth_req)
        auth_resp = _recv_exact(sock, 2)

        if auth_resp[1] != 0x00:
            return True, f"代理正确拒绝了错误凭证（认证响应码 {auth_resp[1]}）✓"
        else:
            return False, "代理接受了错误凭证（预期被拒绝）✗"
    except ConnectionRefusedError:
        return False, f"连接被拒绝：{proxy_host}:{proxy_port}"
    except socket.timeout:
        return False, f"超时（{timeout}s）"
    except Socks5Error as e:
        # 认证失败后代理可能直接断开连接，也视为"正确拒绝"
        return True, f"代理关闭了连接（认证失败行为符合预期）：{e} ✓"
    except Exception as e:
        return False, f"SOCKS5错误认证测试异常：{e}"
    finally:
        if sock:
            try:
                sock.close()
            except Exception:
                pass


def test_socks5_tunnel(
    proxy_host: str,
    proxy_port: int,
    target_host: str,
    target_port: int,
    username: str = "",
    password: str = "",
    timeout: float = 10,
) -> tuple[bool, str]:
    """
    通过 SOCKS5 代理连接到目标服务器（TCP echo server），发送测试数据并验证回显。

    Returns:
        (success, message)
    """
    sock = None
    try:
        sock = _socks5_connect(
            proxy_host,
            proxy_port,
            target_host,
            target_port,
            username=username,
            password=password,
            timeout=timeout,
        )

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
            return True, f"SOCKS5握手成功，发送 {len(TEST_DATA)} 字节，回显一致 ✓"
        else:
            return (
                False,
                f"数据不匹配：期望 {len(TEST_DATA)} 字节，实际收到 {len(received)} 字节",
            )
    except ConnectionRefusedError:
        return False, f"连接被拒绝：{proxy_host}:{proxy_port}（SOCKS5入口未监听）"
    except socket.timeout:
        return False, f"连接/接收超时（{timeout}s）"
    except Socks5Error as e:
        return False, f"SOCKS5协议错误：{e}"
    except Exception as e:
        return False, f"SOCKS5测试异常：{e}"
    finally:
        if sock:
            sock.close()

