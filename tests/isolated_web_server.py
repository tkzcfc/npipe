"""Helpers to boot an isolated np_server for admin API tests.

The process uses a temporary SQLite database and loopback ports only.
It never touches the project data.db or any production config.
"""

from __future__ import annotations

import json
import os
import secrets
import socket
import subprocess
import tempfile
import time
from pathlib import Path
from typing import Optional

import requests

ROOT_DIR = Path(__file__).resolve().parents[1]


def free_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


def binary_path(name: str) -> Path:
    exe = ".exe" if os.name == "nt" else ""
    return ROOT_DIR / "target" / "debug" / f"{name}{exe}"


def ensure_binary(path: Path, package: str, build: bool) -> None:
    if path.exists():
        return
    if not build:
        raise RuntimeError(f"{path} does not exist; build {package} or pass --build")
    subprocess.run(["cargo", "build", "-p", package], cwd=ROOT_DIR, check=True)


class IsolatedServer:
    def __init__(self, build: bool = False, keep_temp: bool = False):
        self.build = build
        self.keep_temp = keep_temp
        self.temp_root: Optional[Path] = None
        self.process: Optional[subprocess.Popen] = None
        self.web_port = 0
        self.tcp_port = 0
        self.base_url = ""
        self.admin_username = "listq_admin"
        self.admin_password = secrets.token_urlsafe(16)
        self.log_path: Optional[Path] = None

    def start(self, timeout: float = 20.0) -> None:
        server_bin = binary_path("np_server")
        ensure_binary(server_bin, "np_server", self.build)
        self.tcp_port = free_port()
        self.web_port = free_port()
        self.temp_root = Path(tempfile.mkdtemp(prefix="npipe-list-query-"))
        db_path = self.temp_root / "test.db"
        config_path = self.temp_root / "server.json"
        self.log_path = self.temp_root / "server.stdout.log"
        log_dir = self.temp_root / "logs"
        log_dir.mkdir(parents=True, exist_ok=True)
        config = {
            "database_url": f"sqlite://{db_path.as_posix()}?mode=rwc",
            "listen_addr": f"tcp://127.0.0.1:{self.tcp_port}",
            "illegal_traffic_forward": "",
            "illegal_traffic_forward_rules": [],
            "enable_tls": False,
            "tls_cert": "",
            "tls_key": "",
            "web_base_dir": "./dist",
            "web_addr": f"127.0.0.1:{self.web_port}",
            "web_enable_tls": False,
            "web_tls_cert": "",
            "web_tls_key": "",
            "web_tls_auto_self_signed": False,
            "web_cookie_secure": False,
            "web_username": self.admin_username,
            "web_password": self.admin_password,
            "web_allowed_origins": [],
            "quiet": True,
            "log_dir": log_dir.as_posix(),
        }
        config_path.write_text(json.dumps(config, ensure_ascii=False, indent=2), encoding="utf-8")
        self._log_file = self.log_path.open("w", encoding="utf-8")
        self.process = subprocess.Popen(
            [str(server_bin), "--log-level", "info", "-c", str(config_path)],
            cwd=ROOT_DIR,
            stdout=self._log_file,
            stderr=subprocess.STDOUT,
            text=True,
        )
        self.base_url = f"http://127.0.0.1:{self.web_port}"
        deadline = time.time() + timeout
        last_error = None
        session = requests.Session()
        while time.time() < deadline:
            if self.process.poll() is not None:
                raise RuntimeError(f"np_server exited early; see {self.log_path}")
            try:
                resp = session.post(
                    f"{self.base_url}/api/login",
                    json={"username": self.admin_username, "password": self.admin_password},
                    timeout=1,
                )
                data = resp.json()
                if data.get("code") == 0:
                    return
                last_error = data.get("msg")
            except (requests.RequestException, ValueError) as exc:
                last_error = str(exc)
            time.sleep(0.15)
        raise RuntimeError(f"admin API did not become ready: {last_error}")

    def stop(self) -> None:
        if self.process and self.process.poll() is None:
            self.process.terminate()
            try:
                self.process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                self.process.kill()
                self.process.wait(timeout=5)
        log_file = getattr(self, "_log_file", None)
        if log_file and not log_file.closed:
            log_file.close()
        if self.temp_root and not self.keep_temp:
            try:
                import shutil
                shutil.rmtree(self.temp_root, ignore_errors=True)
            except OSError:
                pass


class ApiClient:
    def __init__(self, base_url: str, username: str, password: str):
        self.base_url = base_url.rstrip("/")
        self.username = username
        self.password = password
        self.session = requests.Session()

    def login(self) -> dict:
        resp = self.session.post(
            f"{self.base_url}/api/login",
            json={"username": self.username, "password": self.password},
            timeout=10,
        )
        resp.raise_for_status()
        data = resp.json()
        if data.get("code") != 0:
            raise RuntimeError(f"login failed for {self.username}: {data.get('msg')}")
        return data

    def post(self, path: str, payload: dict) -> dict:
        resp = self.session.post(f"{self.base_url}{path}", json=payload, timeout=15)
        resp.raise_for_status()
        return resp.json()
