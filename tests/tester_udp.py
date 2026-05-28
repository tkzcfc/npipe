"""
UDP 隧道测试
"""
import socket
import logging
import os
import time

logger = logging.getLogger("tester.udp")

TEST_DATA = b"Hello npipe UDP tunnel! " + os.urandom(16)
RETRY_COUNT = 5


def test_udp_tunnel(
    inlet_host: str,
    inlet_port: int,
    timeout: float = 10,
) -> tuple[bool, str]:
    """
    通过 UDP 隧道发送测试数据包，验证回显一致。
    由于 UDP 存在丢包，会重试多次。

    Returns:
        (success, message)
    """
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.settimeout(timeout / RETRY_COUNT)
    try:
        target = (inlet_host, inlet_port)
        for attempt in range(1, RETRY_COUNT + 1):
            try:
                sock.sendto(TEST_DATA, target)
                data, _ = sock.recvfrom(4096)
                if data == TEST_DATA:
                    return True, f"第{attempt}次尝试成功，发送 {len(TEST_DATA)} 字节，回显一致 ✓"
                else:
                    return (
                        False,
                        f"数据不匹配：期望 {len(TEST_DATA)} 字节，实际收到 {len(data)} 字节",
                    )
            except socket.timeout:
                logger.debug(f"UDP 第{attempt}次尝试超时，重试...")
                time.sleep(0.2)
            except Exception as e:
                return False, f"UDP测试异常：{e}"

        return False, f"UDP 测试失败：{RETRY_COUNT}次尝试均超时（入口未监听或隧道未就绪）"
    finally:
        sock.close()

