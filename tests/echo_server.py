"""
简单的TCP/UDP回显服务器，用于测试代理隧道是否正常工作
"""
import socket
import threading
import logging
import time

logger = logging.getLogger("echo_server")


class TCPEchoServer:
    """TCP 回显服务器：原样返回收到的所有数据"""

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
        logger.info(f"TCP Echo Server started on {self.host}:{self.port}")

    def stop(self):
        self._stop_event.set()
        if self._server_socket:
            try:
                self._server_socket.close()
            except Exception:
                pass
        if self._thread:
            self._thread.join(timeout=3)
        logger.info("TCP Echo Server stopped")

    def _serve(self):
        while not self._stop_event.is_set():
            try:
                conn, addr = self._server_socket.accept()
                t = threading.Thread(
                    target=self._handle_client, args=(conn, addr), daemon=True
                )
                t.start()
            except socket.timeout:
                continue
            except OSError:
                break

    def _handle_client(self, conn: socket.socket, addr):
        logger.debug(f"TCP Echo: connection from {addr}")
        try:
            conn.settimeout(30)
            while True:
                data = conn.recv(4096)
                if not data:
                    break
                conn.sendall(data)
        except Exception as e:
            logger.debug(f"TCP Echo client {addr} error: {e}")
        finally:
            conn.close()


class UDPEchoServer:
    """UDP 回显服务器：原样返回收到的所有数据包"""

    def __init__(self, host: str, port: int):
        self.host = host
        self.port = port
        self._socket: socket.socket | None = None
        self._thread: threading.Thread | None = None
        self._stop_event = threading.Event()

    def start(self):
        self._socket = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        self._socket.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        self._socket.bind((self.host, self.port))
        self._socket.settimeout(1.0)
        self._stop_event.clear()
        self._thread = threading.Thread(target=self._serve, daemon=True)
        self._thread.start()
        logger.info(f"UDP Echo Server started on {self.host}:{self.port}")

    def stop(self):
        self._stop_event.set()
        if self._socket:
            try:
                self._socket.close()
            except Exception:
                pass
        if self._thread:
            self._thread.join(timeout=3)
        logger.info("UDP Echo Server stopped")

    def _serve(self):
        while not self._stop_event.is_set():
            try:
                data, addr = self._socket.recvfrom(4096)
                if data:
                    self._socket.sendto(data, addr)
                    logger.debug(f"UDP Echo: {len(data)} bytes from {addr}")
            except socket.timeout:
                continue
            except OSError:
                break

