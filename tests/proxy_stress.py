#!/usr/bin/env python3
"""
Proxy stress test for npipe.

Tests multi-connection proxy functionality under load:
1. Start np_server + np_client
2. Create a TCP tunnel (inlet -> echo server)
3. Open many concurrent connections through the tunnel
4. Send data and verify echo responses
5. Report throughput and error stats
"""

import argparse
import json
import os
import shutil
import socket
import subprocess
import sys
import tempfile
import threading
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path
from typing import Optional

import requests

from admin_api import AdminAPI
from echo_server import TCPEchoServer

ROOT_DIR = Path(__file__).resolve().parents[1]


def _free_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


def _wait_for_admin(api: AdminAPI, timeout: float) -> None:
    deadline = time.time() + timeout
    while time.time() < deadline:
        try:
            response = api._session.post(
                f"{api.base_url}/api/login",
                json={"username": api.username, "password": api.password},
                timeout=1,
            )
            data = response.json()
            if data.get("code") == 0:
                api._logged_in = True
                return
        except (requests.RequestException, ValueError):
            pass
        time.sleep(0.25)
    raise RuntimeError("admin API did not become ready")


def _read_text(path: Path) -> str:
    if not path.exists():
        return ""
    return path.read_text(encoding="utf-8", errors="replace")


def _binary_path(default_name: str, override: Optional[str]) -> Path:
    if override:
        return Path(override)
    exe = ".exe" if os.name == "nt" else ""
    return ROOT_DIR / "target" / "debug" / f"{default_name}{exe}"


def _ensure_binary(path: Path, package: str, build: bool) -> None:
    if path.exists():
        return
    if not build:
        raise RuntimeError(f"{path} does not exist; run cargo build or pass --build")
    subprocess.run(["cargo", "build", "-p", package], cwd=ROOT_DIR, check=True)


def _write_server_config(
    path: Path, db_path: Path, tcp_port: int, web_port: int,
    log_dir: Path, max_connections: int, idle_timeout_secs: int,
) -> None:
    config = {
        "database_url": f"sqlite://{db_path.as_posix()}?mode=rwc",
        "listen_addr": f"tcp://127.0.0.1:{tcp_port}",
        "illegal_traffic_forward": "",
        "illegal_traffic_forward_rules": [],
        "enable_tls": False,
        "tls_cert": "./cert.pem",
        "tls_key": "./server.key.pem",
        "web_base_dir": "./dist",
        "web_addr": f"127.0.0.1:{web_port}",
        "web_enable_tls": False,
        "web_tls_cert": "",
        "web_tls_key": "",
        "web_tls_auto_self_signed": False,
        "web_cookie_secure": False,
        "web_username": "stress_admin",
        "web_password": "stress_pass",
        "transport_max_connections_per_player": max_connections,
        "transport_idle_timeout_secs": idle_timeout_secs,
        "transport_token_ttl_secs": 30,
        "quiet": False,
        "log_dir": log_dir.as_posix(),
    }
    path.write_text(json.dumps(config, ensure_ascii=False, indent=2), encoding="utf-8")


def _start_server(server_bin: Path, config_path: Path, log_path: Path) -> subprocess.Popen:
    log_file = log_path.open("w", encoding="utf-8")
    return subprocess.Popen(
        [str(server_bin), "--log-level", "debug", "-c", str(config_path)],
        cwd=ROOT_DIR, stdout=log_file, stderr=subprocess.STDOUT, text=True,
    )


def _stop_process(process: subprocess.Popen, timeout: float = 5) -> None:
    if process.poll() is not None:
        return
    process.terminate()
    try:
        process.wait(timeout=timeout)
    except subprocess.TimeoutExpired:
        process.kill()
        process.wait(timeout=timeout)


def _wait_for_log(path: Path, needle: str, timeout: float) -> bool:
    deadline = time.time() + timeout
    while time.time() < deadline:
        if needle in _read_text(path):
            return True
        time.sleep(0.25)
    return False


def _wait_for_port(host: str, port: int, timeout: float) -> bool:
    """Wait until a TCP port is accepting connections."""
    deadline = time.time() + timeout
    while time.time() < deadline:
        try:
            with socket.create_connection((host, port), timeout=0.5):
                return True
        except (ConnectionRefusedError, OSError, socket.timeout):
            time.sleep(0.2)
    return False


def _single_echo_test(inlet_host: str, inlet_port: int, payload: bytes, timeout: float) -> dict:
    """Connect to inlet, send payload, verify echo. Returns stats dict."""
    result = {"ok": False, "bytes_sent": 0, "bytes_recv": 0, "latency_ms": 0.0, "error": ""}
    try:
        t0 = time.perf_counter()
        with socket.create_connection((inlet_host, inlet_port), timeout=timeout) as sock:
            sock.sendall(payload)
            result["bytes_sent"] = len(payload)

            received = b""
            sock.settimeout(timeout)
            while len(received) < len(payload):
                chunk = sock.recv(65536)
                if not chunk:
                    break
                received += chunk

            t1 = time.perf_counter()
            result["latency_ms"] = (t1 - t0) * 1000
            result["bytes_recv"] = len(received)

            if received == payload:
                result["ok"] = True
            else:
                result["error"] = f"data mismatch: sent {len(payload)}B, got {len(received)}B"
    except Exception as e:
        result["error"] = str(e)
    return result


def run_stress(args: argparse.Namespace) -> None:
    server_bin = _binary_path("np_server", args.server_bin)
    client_bin = _binary_path("np_client", args.client_bin)
    _ensure_binary(server_bin, "np_server", args.build)
    _ensure_binary(client_bin, "np_client", args.build)

    tcp_port = _free_port()
    web_port = _free_port()
    echo_port = _free_port()
    inlet_port = _free_port()

    temp_root = Path(args.temp_dir) if args.temp_dir else Path(
        tempfile.mkdtemp(prefix="npipe-proxy-stress-")
    )
    temp_root.mkdir(parents=True, exist_ok=True)

    db_path = temp_root / "stress.db"
    config_path = temp_root / "server.json"
    server_log = temp_root / "server.log"
    client_log = temp_root / "client.log"
    log_dir = temp_root / "logs"
    log_dir.mkdir(parents=True, exist_ok=True)

    _write_server_config(
        config_path, db_path, tcp_port, web_port, log_dir,
        args.max_connections, args.idle_timeout_secs,
    )

    # Start echo server
    echo = TCPEchoServer("127.0.0.1", echo_port)
    echo.start()

    server = _start_server(server_bin, config_path, server_log)
    try:
        api = AdminAPI(
            base_url=f"http://127.0.0.1:{web_port}",
            username="stress_admin",
            password="stress_pass",
        )
        _wait_for_admin(api, 15)

        # Create player
        existing = api.find_player_by_username(args.username)
        if existing is None:
            result = api.add_player(args.username, args.password)
            if result.get("code") != 0:
                raise RuntimeError(f"add_player failed: {result}")

        player = api.find_player_by_username(args.username)
        player_id = player["id"]

        # Create TCP tunnel: inlet_port -> echo_port (same player as sender+receiver)
        tunnel_result = api.add_tunnel(
            source=f"127.0.0.1:{inlet_port}",
            endpoint=f"127.0.0.1:{echo_port}",
            tunnel_type=0,  # TCP
            sender=player_id,
            receiver=player_id,
            description="stress-test-tunnel",
            enabled=1,
        )
        if tunnel_result.get("code") != 0:
            raise RuntimeError(f"add_tunnel failed: {tunnel_result}")

        # Start client
        client_cmd = [
            str(client_bin), "run",
            "--server", f"tcp://127.0.0.1:{tcp_port}",
            "--username", args.username,
            "--password", args.password,
            "--transport-max-connections", str(args.max_connections),
            "--transport-idle-timeout-secs", str(args.idle_timeout_secs),
            "--log-level", "debug",
            "--base-log-level", "error",
            "--log-dir", (temp_root / "client-logs").as_posix(),
        ]
        client_log_file = client_log.open("w", encoding="utf-8")
        client = subprocess.Popen(
            client_cmd, cwd=ROOT_DIR,
            stdout=client_log_file, stderr=subprocess.STDOUT, text=True,
        )
        try:
            if not _wait_for_log(client_log, "Login successful", 15):
                raise RuntimeError("client did not login")

            # Wait for inlet to start listening
            if not _wait_for_port("127.0.0.1", inlet_port, 10):
                raise RuntimeError(f"inlet port {inlet_port} not listening")

            print(f"Setup complete:")
            print(f"  echo_port={echo_port}, inlet_port={inlet_port}")
            print(f"  max_connections={args.max_connections}")
            print(f"  concurrency={args.concurrency}, rounds={args.rounds}")
            print(f"  payload_size={args.payload_size}B")
            print()

            # Run stress test
            payload = os.urandom(args.payload_size)
            total_ok = 0
            total_fail = 0
            total_bytes = 0
            latencies = []

            t_start = time.perf_counter()

            for round_idx in range(args.rounds):
                futures = []
                with ThreadPoolExecutor(max_workers=args.concurrency) as pool:
                    for _ in range(args.concurrency):
                        futures.append(
                            pool.submit(_single_echo_test, "127.0.0.1", inlet_port, payload, 30)
                        )

                    for f in as_completed(futures):
                        r = f.result()
                        if r["ok"]:
                            total_ok += 1
                            total_bytes += r["bytes_sent"] + r["bytes_recv"]
                            latencies.append(r["latency_ms"])
                        else:
                            total_fail += 1
                            if total_fail <= 5:
                                print(f"  [FAIL] {r['error']}")

                if (round_idx + 1) % max(1, args.rounds // 5) == 0:
                    print(f"  round {round_idx + 1}/{args.rounds}: ok={total_ok} fail={total_fail}")

            t_elapsed = time.perf_counter() - t_start

            # Report
            total_requests = total_ok + total_fail
            print()
            print("=" * 50)
            print(f"Stress test results:")
            print(f"  Total requests:  {total_requests}")
            print(f"  Success:         {total_ok}")
            print(f"  Failed:          {total_fail}")
            print(f"  Success rate:    {total_ok / total_requests * 100:.1f}%")
            print(f"  Elapsed:         {t_elapsed:.2f}s")
            print(f"  Throughput:      {total_requests / t_elapsed:.1f} req/s")
            print(f"  Data transferred:{total_bytes / 1024 / 1024:.2f} MB")
            if latencies:
                latencies.sort()
                print(f"  Latency avg:     {sum(latencies) / len(latencies):.1f}ms")
                print(f"  Latency p50:     {latencies[len(latencies) // 2]:.1f}ms")
                print(f"  Latency p95:     {latencies[int(len(latencies) * 0.95)]:.1f}ms")
                print(f"  Latency p99:     {latencies[int(len(latencies) * 0.99)]:.1f}ms")
                print(f"  Latency max:     {latencies[-1]:.1f}ms")
            print("=" * 50)

            if total_fail > 0:
                fail_rate = total_fail / total_requests
                if fail_rate > 0.05:
                    raise RuntimeError(
                        f"failure rate {fail_rate * 100:.1f}% exceeds 5% threshold"
                    )
                print(f"  WARNING: {total_fail} failures ({fail_rate * 100:.1f}%)")

            if args.keep_temp or args.temp_dir is not None:
                print(f"  temp_dir={temp_root}")

        finally:
            _stop_process(client)
            client_log_file.close()
    finally:
        _stop_process(server)
        echo.stop()
        if not args.keep_temp and args.temp_dir is None:
            shutil.rmtree(temp_root, ignore_errors=True)


def main() -> int:
    parser = argparse.ArgumentParser(description="Run npipe proxy stress test.")
    parser.add_argument("--server-bin", help="Path to np_server binary.")
    parser.add_argument("--client-bin", help="Path to np_client binary.")
    parser.add_argument("--build", action="store_true", help="Build missing debug binaries.")
    parser.add_argument("--max-connections", type=int, default=4,
                        help="Transport max connections (pool size).")
    parser.add_argument("--idle-timeout-secs", type=int, default=30)
    parser.add_argument("--concurrency", type=int, default=20,
                        help="Number of concurrent connections per round.")
    parser.add_argument("--rounds", type=int, default=10,
                        help="Number of rounds to repeat.")
    parser.add_argument("--payload-size", type=int, default=4096,
                        help="Payload size in bytes per request.")
    parser.add_argument("--username", default="stress_user")
    parser.add_argument("--password", default="stresspass1")
    parser.add_argument("--temp-dir", help="Use an explicit temp directory.")
    parser.add_argument("--keep-temp", action="store_true")
    args = parser.parse_args()

    try:
        run_stress(args)
        return 0
    except Exception as exc:
        print(f"proxy stress test failed: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
