#!/usr/bin/env python3
"""
npipe 代理工具测试套件
======================
支持 TCP / UDP / SOCKS5 / HTTP 四种隧道类型的端到端测试，
包括：
  - 基础功能测试（小数据回显）
  - 带用户名/密码的认证测试（SOCKS5 / HTTP）
  - 错误凭证拒绝验证
  - 大数据量传输测试（SHA-256 完整性 + 吞吐量统计）

用法
----
  python proxy_tester.py                             # 使用默认配置 test_config.json
  python proxy_tester.py -c my_config.json           # 指定配置文件
  python proxy_tester.py --type tcp                  # 只测试某种类型
  python proxy_tester.py --type bulk                 # 只跑大数据量测试
  python proxy_tester.py --no-create                 # 不自动创建隧道
  python proxy_tester.py --no-cleanup                # 测试后不删除隧道
  python proxy_tester.py -v                          # 详细日志

依赖安装
--------
  pip install requests colorama
"""

import argparse
import json
import logging
import os
import sys
import time
from dataclasses import dataclass
from typing import Optional

# --------------------------------------------------------------------------- #
# 颜色输出（Windows 兼容）
# --------------------------------------------------------------------------- #
try:
    import colorama
    colorama.init(autoreset=True)
    GREEN  = colorama.Fore.GREEN
    RED    = colorama.Fore.RED
    YELLOW = colorama.Fore.YELLOW
    CYAN   = colorama.Fore.CYAN
    RESET  = colorama.Style.RESET_ALL
    BOLD   = colorama.Style.BRIGHT
except ImportError:
    GREEN = RED = YELLOW = CYAN = RESET = BOLD = ""

# --------------------------------------------------------------------------- #
# 日志
# --------------------------------------------------------------------------- #
logging.basicConfig(
    level=logging.WARNING,
    format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
    datefmt="%H:%M:%S",
)
logger = logging.getLogger("proxy_tester")


# --------------------------------------------------------------------------- #
# 测试结果
# --------------------------------------------------------------------------- #
@dataclass
class TestResult:
    name: str
    success: bool
    message: str
    duration: float = 0.0
    skipped: bool = False

    def colored_status(self) -> str:
        if self.skipped:
            return f"{YELLOW}SKIP{RESET}"
        return f"{GREEN}PASS{RESET}" if self.success else f"{RED}FAIL{RESET}"


# --------------------------------------------------------------------------- #
# 工具函数
# --------------------------------------------------------------------------- #
def _load_config(path: str) -> dict:
    with open(path, "r", encoding="utf-8") as f:
        return json.load(f)


def _wait_for_tcp_port(host: str, port: int, timeout: float = 15) -> bool:
    import socket
    deadline = time.time() + timeout
    while time.time() < deadline:
        try:
            s = socket.create_connection((host, port), timeout=1)
            s.close()
            return True
        except OSError:
            time.sleep(0.5)
    return False


def _print_banner(title: str):
    width = 64
    print(f"\n{BOLD}{CYAN}{'=' * width}{RESET}")
    print(f"{BOLD}{CYAN}{title.center(width)}{RESET}")
    print(f"{BOLD}{CYAN}{'=' * width}{RESET}\n")


def _print_result(result: TestResult):
    status = result.colored_status()
    dur = f"{result.duration:.2f}s"
    print(f"  [{status}] {BOLD}{result.name:<28}{RESET}  {dur:>7}  {result.message}")


# --------------------------------------------------------------------------- #
# 主测试器
# --------------------------------------------------------------------------- #
class ProxyTester:
    def __init__(self, config: dict):
        self.cfg = config
        self.results: list[TestResult] = []
        self._created_tunnel_ids: list[int] = []

        from admin_api import AdminAPI
        admin_cfg = config["admin"]
        self.api = AdminAPI(
            base_url=admin_cfg["url"],
            username=admin_cfg["username"],
            password=admin_cfg["password"],
        )

        from echo_server import TCPEchoServer, UDPEchoServer
        es = config["echo_server"]
        self.tcp_echo = TCPEchoServer(es["tcp_host"], es["tcp_port"])
        self.udp_echo = UDPEchoServer(es["udp_host"], es["udp_port"])

        from tester_bulk import FramedEchoServer
        fe = config["framed_echo_server"]
        self.framed_echo = FramedEchoServer(fe["host"], fe["port"])

    # ------------------------------------------------------------------ #
    # Lifecycle
    # ------------------------------------------------------------------ #
    def _start_echo_servers(self):
        self.tcp_echo.start()
        self.udp_echo.start()
        self.framed_echo.start()
        time.sleep(0.3)

    def _stop_echo_servers(self):
        self.tcp_echo.stop()
        self.udp_echo.stop()
        self.framed_echo.stop()

    def _cleanup_tunnels(self):
        for tid in self._created_tunnel_ids:
            try:
                self.api.remove_tunnel(tid)
                logger.info(f"已删除测试隧道 id={tid}")
            except Exception as e:
                logger.warning(f"删除隧道 id={tid} 失败：{e}")
        self._created_tunnel_ids.clear()

    # ------------------------------------------------------------------ #
    # Tunnel helpers
    # ------------------------------------------------------------------ #
    def _create_tunnel_if_needed(
        self,
        description: str,
        source: str,
        endpoint: str,
        tunnel_type: int,
        username: str = "",
        password: str = "",
    ) -> bool:
        if not self.cfg["test"].get("create_tunnels", True):
            return True

        players = self.cfg["players"]
        sender   = players["sender_id"]
        receiver = players["receiver_id"]

        existing = self.api.find_tunnel_by_description(description)
        if existing:
            logger.info(f"隧道已存在（{description}），跳过创建")
            if existing["id"] not in self._created_tunnel_ids:
                self._created_tunnel_ids.append(existing["id"])
            return True

        result = self.api.add_tunnel(
            source=source, endpoint=endpoint, tunnel_type=tunnel_type,
            sender=sender, receiver=receiver, description=description,
            username=username, password=password,
        )
        if result.get("code") == 0:
            created = self.api.find_tunnel_by_description(description)
            if created:
                self._created_tunnel_ids.append(created["id"])
            logger.info(f"已创建隧道：{description}")
            return True
        else:
            logger.error(f"创建隧道失败：{description}  {result.get('msg')}")
            return False

    def _skip(self, name: str):
        self.results.append(TestResult(name, False, "", skipped=True))

    def _add(self, name: str, t0: float, ok: bool, msg: str):
        self.results.append(TestResult(name, ok, msg, time.time() - t0))

    # ------------------------------------------------------------------ #
    # ① TCP 基础
    # ------------------------------------------------------------------ #
    def run_tcp_test(self):
        tc = self.cfg["tunnels"]["tcp"]
        if not tc.get("enabled", True):
            return self._skip("TCP隧道")

        es = self.cfg["echo_server"]
        if not self._create_tunnel_if_needed(
            "__npipe_test_tcp__",
            f"{tc['inlet_host']}:{tc['inlet_port']}",
            f"{es['tcp_host']}:{es['tcp_port']}", 0,
        ):
            return self.results.append(TestResult("TCP隧道", False, "隧道创建失败"))

        if not _wait_for_tcp_port(tc["inlet_host"], tc["inlet_port"], 15):
            return self._add("TCP隧道", time.time(), False, f"等待入口超时：{tc['inlet_host']}:{tc['inlet_port']}")

        from tester_tcp import test_tcp_tunnel
        t0 = time.time()
        ok, msg = test_tcp_tunnel(tc["inlet_host"], tc["inlet_port"],
                                   timeout=self.cfg["test"]["timeout"])
        self._add("TCP隧道", t0, ok, msg)

    # ------------------------------------------------------------------ #
    # ② UDP 基础
    # ------------------------------------------------------------------ #
    def run_udp_test(self):
        tc = self.cfg["tunnels"]["udp"]
        if not tc.get("enabled", True):
            return self._skip("UDP隧道")

        es = self.cfg["echo_server"]
        if not self._create_tunnel_if_needed(
            "__npipe_test_udp__",
            f"{tc['inlet_host']}:{tc['inlet_port']}",
            f"{es['udp_host']}:{es['udp_port']}", 1,
        ):
            return self.results.append(TestResult("UDP隧道", False, "隧道创建失败"))

        time.sleep(1.0)

        from tester_udp import test_udp_tunnel
        t0 = time.time()
        ok, msg = test_udp_tunnel(tc["inlet_host"], tc["inlet_port"],
                                   timeout=self.cfg["test"]["timeout"])
        self._add("UDP隧道", t0, ok, msg)

    # ------------------------------------------------------------------ #
    # ③ SOCKS5 无认证
    # ------------------------------------------------------------------ #
    def run_socks5_test(self):
        tc = self.cfg["tunnels"]["socks5"]
        if not tc.get("enabled", True):
            return self._skip("SOCKS5(无认证)")

        if not self._create_tunnel_if_needed(
            "__npipe_test_socks5__",
            f"{tc['inlet_host']}:{tc['inlet_port']}", "", 2,
        ):
            return self.results.append(TestResult("SOCKS5(无认证)", False, "隧道创建失败"))

        if not _wait_for_tcp_port(tc["inlet_host"], tc["inlet_port"], 15):
            return self._add("SOCKS5(无认证)", time.time(), False, "等待入口超时")

        tgt = self.cfg["test"]["socks5_test_target"]
        from tester_socks5 import test_socks5_tunnel
        t0 = time.time()
        ok, msg = test_socks5_tunnel(
            tc["inlet_host"], tc["inlet_port"],
            tgt["host"], tgt["port"],
            timeout=self.cfg["test"]["timeout"],
        )
        self._add("SOCKS5(无认证)", t0, ok, msg)

    # ------------------------------------------------------------------ #
    # ④ SOCKS5 带认证（正确凭证 + 错误凭证）
    # ------------------------------------------------------------------ #
    def run_socks5_auth_test(self):
        tc = self.cfg["tunnels"]["socks5_auth"]
        if not tc.get("enabled", True):
            self._skip("SOCKS5(正确凭证)")
            self._skip("SOCKS5(错误凭证拒绝)")
            return

        uname = tc.get("username", "")
        passwd = tc.get("password", "")

        if not self._create_tunnel_if_needed(
            "__npipe_test_socks5_auth__",
            f"{tc['inlet_host']}:{tc['inlet_port']}", "", 2,
            username=uname, password=passwd,
        ):
            self.results.append(TestResult("SOCKS5(正确凭证)", False, "隧道创建失败"))
            self.results.append(TestResult("SOCKS5(错误凭证拒绝)", False, "隧道创建失败"))
            return

        if not _wait_for_tcp_port(tc["inlet_host"], tc["inlet_port"], 15):
            msg = "等待入口超时"
            self.results.append(TestResult("SOCKS5(正确凭证)", False, msg))
            self.results.append(TestResult("SOCKS5(错误凭证拒绝)", False, msg))
            return

        tgt = self.cfg["test"]["socks5_test_target"]
        from tester_socks5 import test_socks5_tunnel, test_socks5_wrong_auth

        # 正确凭证
        t0 = time.time()
        ok, msg = test_socks5_tunnel(
            tc["inlet_host"], tc["inlet_port"],
            tgt["host"], tgt["port"],
            username=uname, password=passwd,
            timeout=self.cfg["test"]["timeout"],
        )
        self._add("SOCKS5(正确凭证)", t0, ok, msg)

        # 错误凭证应被拒绝
        t0 = time.time()
        ok, msg = test_socks5_wrong_auth(
            tc["inlet_host"], tc["inlet_port"],
            tgt["host"], tgt["port"],
            timeout=self.cfg["test"]["timeout"],
        )
        self._add("SOCKS5(错误凭证拒绝)", t0, ok, msg)

    # ------------------------------------------------------------------ #
    # ⑤ HTTP 无认证
    # ------------------------------------------------------------------ #
    def run_http_test(self):
        tc = self.cfg["tunnels"]["http"]
        if not tc.get("enabled", True):
            return self._skip("HTTP(无认证)")

        if not self._create_tunnel_if_needed(
            "__npipe_test_http__",
            f"{tc['inlet_host']}:{tc['inlet_port']}", "", 3,
        ):
            return self.results.append(TestResult("HTTP(无认证)", False, "隧道创建失败"))

        if not _wait_for_tcp_port(tc["inlet_host"], tc["inlet_port"], 15):
            return self._add("HTTP(无认证)", time.time(), False, "等待入口超时")

        tgt = self.cfg["test"]["http_test_target"]
        from tester_http import test_http_connect_tunnel
        t0 = time.time()
        ok, msg = test_http_connect_tunnel(
            tc["inlet_host"], tc["inlet_port"],
            tgt["host"], tgt["port"],
            timeout=self.cfg["test"]["timeout"],
        )
        self._add("HTTP(无认证)", t0, ok, msg)

    # ------------------------------------------------------------------ #
    # ⑥ HTTP 带认证（正确凭证 + 错误凭证）
    # ------------------------------------------------------------------ #
    def run_http_auth_test(self):
        tc = self.cfg["tunnels"]["http_auth"]
        if not tc.get("enabled", True):
            self._skip("HTTP(正确凭证)")
            self._skip("HTTP(错误凭证拒绝)")
            return

        uname = tc.get("username", "")
        passwd = tc.get("password", "")

        if not self._create_tunnel_if_needed(
            "__npipe_test_http_auth__",
            f"{tc['inlet_host']}:{tc['inlet_port']}", "", 3,
            username=uname, password=passwd,
        ):
            self.results.append(TestResult("HTTP(正确凭证)", False, "隧道创建失败"))
            self.results.append(TestResult("HTTP(错误凭证拒绝)", False, "隧道创建失败"))
            return

        if not _wait_for_tcp_port(tc["inlet_host"], tc["inlet_port"], 15):
            msg = "等待入口超时"
            self.results.append(TestResult("HTTP(正确凭证)", False, msg))
            self.results.append(TestResult("HTTP(错误凭证拒绝)", False, msg))
            return

        tgt = self.cfg["test"]["http_test_target"]
        from tester_http import test_http_connect_tunnel, test_http_connect_wrong_auth

        # 正确凭证
        t0 = time.time()
        ok, msg = test_http_connect_tunnel(
            tc["inlet_host"], tc["inlet_port"],
            tgt["host"], tgt["port"],
            username=uname, password=passwd,
            timeout=self.cfg["test"]["timeout"],
        )
        self._add("HTTP(正确凭证)", t0, ok, msg)

        # 错误凭证应被拒绝（407）
        t0 = time.time()
        ok, msg = test_http_connect_wrong_auth(
            tc["inlet_host"], tc["inlet_port"],
            tgt["host"], tgt["port"],
            timeout=self.cfg["test"]["timeout"],
        )
        self._add("HTTP(错误凭证拒绝)", t0, ok, msg)

    # ------------------------------------------------------------------ #
    # ⑦ TCP 大数据量
    # ------------------------------------------------------------------ #
    def run_bulk_tcp_test(self):
        tc = self.cfg["tunnels"]["tcp_bulk"]
        if not tc.get("enabled", True):
            return self._skip("TCP大数据量")

        bk = self.cfg["test"]["bulk"]
        fe = bk["framed_target"]

        if not self._create_tunnel_if_needed(
            "__npipe_test_tcp_bulk__",
            f"{tc['inlet_host']}:{tc['inlet_port']}",
            f"{fe['host']}:{fe['port']}", 0,
        ):
            return self.results.append(TestResult("TCP大数据量", False, "隧道创建失败"))

        if not _wait_for_tcp_port(tc["inlet_host"], tc["inlet_port"], 20):
            return self._add("TCP大数据量", time.time(), False, "等待入口超时")

        from tester_bulk import test_bulk_tcp
        total = int(bk["total_size_mb"] * 1024 * 1024)
        chunk = int(bk["chunk_size_kb"] * 1024)
        t0 = time.time()
        ok, msg = test_bulk_tcp(
            tc["inlet_host"], tc["inlet_port"],
            total_size=total, chunk_size=chunk,
            rounds=bk["rounds"], timeout=bk["timeout"],
        )
        self._add("TCP大数据量", t0, ok, msg)

    # ------------------------------------------------------------------ #
    # ⑧ SOCKS5 大数据量
    # ------------------------------------------------------------------ #
    def run_bulk_socks5_test(self):
        tc = self.cfg["tunnels"]["socks5_bulk"]
        if not tc.get("enabled", True):
            return self._skip("SOCKS5大数据量")

        bk = self.cfg["test"]["bulk"]
        fe = bk["framed_target"]
        uname = tc.get("username", "")
        passwd = tc.get("password", "")

        if not self._create_tunnel_if_needed(
            "__npipe_test_socks5_bulk__",
            f"{tc['inlet_host']}:{tc['inlet_port']}", "", 2,
            username=uname, password=passwd,
        ):
            return self.results.append(TestResult("SOCKS5大数据量", False, "隧道创建失败"))

        if not _wait_for_tcp_port(tc["inlet_host"], tc["inlet_port"], 20):
            return self._add("SOCKS5大数据量", time.time(), False, "等待入口超时")

        from tester_bulk import test_bulk_socks5
        total = int(bk["total_size_mb"] * 1024 * 1024)
        chunk = int(bk["chunk_size_kb"] * 1024)
        t0 = time.time()
        ok, msg = test_bulk_socks5(
            tc["inlet_host"], tc["inlet_port"],
            fe["host"], fe["port"],
            username=uname, password=passwd,
            total_size=total, chunk_size=chunk,
            rounds=bk["rounds"], timeout=bk["timeout"],
        )
        self._add("SOCKS5大数据量", t0, ok, msg)

    # ------------------------------------------------------------------ #
    # ⑨ HTTP 大数据量
    # ------------------------------------------------------------------ #
    def run_bulk_http_test(self):
        tc = self.cfg["tunnels"]["http_bulk"]
        if not tc.get("enabled", True):
            return self._skip("HTTP大数据量")

        bk = self.cfg["test"]["bulk"]
        fe = bk["framed_target"]
        uname = tc.get("username", "")
        passwd = tc.get("password", "")

        if not self._create_tunnel_if_needed(
            "__npipe_test_http_bulk__",
            f"{tc['inlet_host']}:{tc['inlet_port']}", "", 3,
            username=uname, password=passwd,
        ):
            return self.results.append(TestResult("HTTP大数据量", False, "隧道创建失败"))

        if not _wait_for_tcp_port(tc["inlet_host"], tc["inlet_port"], 20):
            return self._add("HTTP大数据量", time.time(), False, "等待入口超时")

        from tester_bulk import test_bulk_http
        total = int(bk["total_size_mb"] * 1024 * 1024)
        chunk = int(bk["chunk_size_kb"] * 1024)
        t0 = time.time()
        ok, msg = test_bulk_http(
            tc["inlet_host"], tc["inlet_port"],
            fe["host"], fe["port"],
            username=uname, password=passwd,
            total_size=total, chunk_size=chunk,
            rounds=bk["rounds"], timeout=bk["timeout"],
        )
        self._add("HTTP大数据量", t0, ok, msg)

    # ------------------------------------------------------------------ #
    # 汇总输出
    # ------------------------------------------------------------------ #
    def _print_summary(self) -> bool:
        _print_banner("测试结果汇总")
        passed  = sum(1 for r in self.results if r.success)
        failed  = sum(1 for r in self.results if not r.success and not r.skipped)
        skipped = sum(1 for r in self.results if r.skipped)
        total   = len(self.results)

        for r in self.results:
            _print_result(r)

        print()
        print(
            f"  总计 {total} 项：{GREEN}{passed} 通过{RESET}  "
            f"{RED}{failed} 失败{RESET}  {YELLOW}{skipped} 跳过{RESET}"
        )
        print()
        return failed == 0

    # ------------------------------------------------------------------ #
    # 入口
    # ------------------------------------------------------------------ #
    #: 所有组的名字 → 对应的方法
    _GROUP_MAP = {
        "tcp":         "run_tcp_test",
        "udp":         "run_udp_test",
        "socks5":      "run_socks5_test",
        "socks5_auth": "run_socks5_auth_test",
        "http":        "run_http_test",
        "http_auth":   "run_http_auth_test",
        "bulk":        None,           # 特殊：展开为三个子测试
    }

    def run(self, types: Optional[list[str]] = None) -> bool:
        _print_banner("npipe 代理工具测试套件")

        # 检查 Admin API
        print(f"  Admin API: {self.cfg['admin']['url']}")
        if not self.api.login():
            print(f"{RED}  ✗ 无法连接 Admin API，请确认 np_server 已启动{RESET}")
            return False
        print(f"{GREEN}  ✓ Admin API 登录成功{RESET}\n")

        # 检查玩家在线状态
        if self.cfg["test"].get("create_tunnels", True):
            players = self.api.list_players()
            sid = self.cfg["players"]["sender_id"]
            rid = self.cfg["players"]["receiver_id"]
            seen = set()
            for p in players:
                if p["id"] in (sid, rid) and p["id"] not in seen:
                    seen.add(p["id"])
                    status = f"{GREEN}在线{RESET}" if p["online"] else f"{YELLOW}离线（隧道可能无法就绪）{RESET}"
                    role   = "sender" if p["id"] == sid else "receiver"
                    print(f"  玩家 [{role}] id={p['id']} {p['username']}：{status}")
            print()

        # 启动 Echo 服务器
        self._start_echo_servers()
        time.sleep(0.5)

        # 确定要执行哪些组
        all_groups = ["tcp", "udp", "socks5", "socks5_auth", "http", "http_auth", "bulk"]
        run_groups = [t.lower() for t in types] if types else all_groups

        try:
            group_labels = {
                "tcp":         "TCP 隧道",
                "udp":         "UDP 隧道",
                "socks5":      "SOCKS5 代理（无认证）",
                "socks5_auth": "SOCKS5 代理（含认证）",
                "http":        "HTTP 代理（无认证）",
                "http_auth":   "HTTP 代理（含认证）",
                "bulk":        "大数据量传输",
            }
            bulk_methods = [
                ("tcp_bulk",    self.run_bulk_tcp_test),
                ("socks5_bulk", self.run_bulk_socks5_test),
                ("http_bulk",   self.run_bulk_http_test),
            ]
            for grp in all_groups:
                if grp not in run_groups:
                    continue
                print(f"{BOLD}[ {group_labels[grp]} ]{RESET}")
                if grp == "bulk":
                    for _, fn in bulk_methods:
                        fn()
                else:
                    getattr(self, self._GROUP_MAP[grp])()
        finally:
            self._stop_echo_servers()
            if self.cfg["test"].get("cleanup_after_test", True):
                self._cleanup_tunnels()
            self.api.logout()

        return self._print_summary()


# --------------------------------------------------------------------------- #
# CLI
# --------------------------------------------------------------------------- #
def main():
    parser = argparse.ArgumentParser(
        description="npipe 代理工具端到端测试套件",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__,
    )
    parser.add_argument(
        "-c", "--config",
        default=os.path.join(os.path.dirname(__file__), "test_config.json"),
        help="配置文件路径（默认：test_config.json）",
    )
    parser.add_argument(
        "--type",
        dest="types",
        nargs="+",
        choices=["tcp", "udp", "socks5", "socks5_auth", "http", "http_auth", "bulk"],
        metavar="TYPE",
        help=(
            "只测试指定类型（可多选）：\n"
            "  tcp / udp / socks5 / socks5_auth / http / http_auth / bulk"
        ),
    )
    parser.add_argument("--no-create",  action="store_true", help="不自动创建隧道")
    parser.add_argument("--no-cleanup", action="store_true", help="测试后不删除隧道")
    parser.add_argument("-v", "--verbose", action="store_true", help="输出详细日志")
    args = parser.parse_args()

    if args.verbose:
        logging.getLogger().setLevel(logging.DEBUG)

    if not os.path.exists(args.config):
        print(f"{RED}配置文件不存在：{args.config}{RESET}")
        sys.exit(1)

    cfg = _load_config(args.config)
    if args.no_create:
        cfg["test"]["create_tunnels"] = False
    if args.no_cleanup:
        cfg["test"]["cleanup_after_test"] = False

    tester = ProxyTester(cfg)
    success = tester.run(types=args.types)
    sys.exit(0 if success else 1)


if __name__ == "__main__":
    main()
