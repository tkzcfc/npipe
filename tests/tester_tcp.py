"""
TCP 隧道测试
"""
import socket
import logging
import os
import time

logger = logging.getLogger("tester.tcp")

TEST_DATA = b"Hello npipe TCP tunnel! " + os.urandom(32)


def test_tcp_tunnel(
    inlet_host: str,
    inlet_port: int,
    timeout: float = 10,
) -> tuple[bool, str]:
    """
    通过 TCP 隧道发送测试数据，验证数据能被回显服务器原样返回。

    Returns:
        (success, message)
    """
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.settimeout(timeout)
    try:
        sock.connect((inlet_host, inlet_port))

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
            return True, f"发送 {len(TEST_DATA)} 字节，回显一致 ✓"
        else:
            return (
                False,
                f"数据不匹配：期望 {len(TEST_DATA)} 字节，实际收到 {len(received)} 字节",
            )
    except ConnectionRefusedError:
        return False, f"连接被拒绝：{inlet_host}:{inlet_port}（入口未监听或隧道未就绪）"
    except socket.timeout:
        return False, f"连接/接收超时（{timeout}s）"
    except Exception as e:
        return False, f"TCP测试异常：{e}"
    finally:
        sock.close()

