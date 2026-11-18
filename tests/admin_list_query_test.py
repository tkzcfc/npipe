#!/usr/bin/env python3
"""Persistent isolated API tests for player_list / tunnel_list query extensions.

Starts a temporary np_server bound to 127.0.0.1 with a fresh SQLite database.
Does not read or modify production data, secrets, or the repo data.db.

Run from the repository root after building np_server:

    cargo build -p np_server
    python tests/admin_list_query_test.py

Use --build to compile np_server if the debug binary is missing.
"""

from __future__ import annotations

import argparse
import os
import secrets
import sys
import time
import unittest
from pathlib import Path

ROOT_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(Path(__file__).resolve().parent))

from isolated_web_server import ApiClient, IsolatedServer, binary_path, ensure_binary  # noqa: E402


LIST_QUERY_VERSION = 1


class ListQueryTests(unittest.TestCase):
    server: IsolatedServer
    admin: ApiClient
    users: dict
    passwords: dict
    tunnels: dict

    @classmethod
    def setUpClass(cls) -> None:
        build = os.environ.get("NPIPE_TEST_BUILD") == "1"
        cls.server = IsolatedServer(build=build)
        cls.server.start()
        cls.admin = ApiClient(cls.server.base_url, cls.server.admin_username, cls.server.admin_password)
        cls.admin.login()
        cls.passwords = {}
        cls.users = {}
        cls.tunnels = {}
        cls._seed()

    @classmethod
    def tearDownClass(cls) -> None:
        cls.server.stop()

    @classmethod
    def _post_admin(cls, path: str, payload: dict) -> dict:
        return cls.admin.post(path, payload)

    @classmethod
    def _seed(cls) -> None:
        # 25 regular users so pagination can cross a 20-item page.
        for i in range(25):
            username = f"user{i:02d}"
            password = secrets.token_hex(6)
            cls.passwords[username] = password
            result = cls._post_admin("/api/add_player", {"username": username, "password": password})
            if result.get("code") != 0:
                raise RuntimeError(f"add_player {username} failed: {result}")

        special_name = "literal%_name"
        special_pass = secrets.token_hex(6)
        cls.passwords[special_name] = special_pass
        result = cls._post_admin("/api/add_player", {"username": special_name, "password": special_pass})
        if result.get("code") != 0:
            raise RuntimeError(f"add_player special failed: {result}")

        # Reload full user map
        listed = cls._post_admin("/api/player_list", {"page_number": 0, "page_size": 100, "q": "user"})
        for item in listed.get("players", []):
            cls.users[item["username"]] = item
        special_list = cls._post_admin("/api/player_list", {"page_number": 0, "page_size": 20, "q": "literal%_name"})
        for item in special_list.get("players", []):
            cls.users[item["username"]] = item

        # Disable a few and grant web access to two accounts.
        disabled_names = [f"user{i:02d}" for i in range(20, 25)]
        for name in disabled_names:
            cls._post_admin("/api/update_player_status", {"id": cls.users[name]["id"], "enabled": 0})
        for name in ("user00", "user01"):
            cls._post_admin("/api/update_player_web_access", {"id": cls.users[name]["id"], "web_access": 1})

        alice_id = cls.users["user00"]["id"]
        bob_id = cls.users["user01"]["id"]
        carol_id = cls.users["user02"]["id"]

        # 25 TCP tunnels (receiver = server) so list pagination crosses 20.
        for i in range(25):
            source_port = 18000 + i
            desc = f"tcp-bulk-{i:02d}"
            result = cls._post_admin("/api/add_tunnel", {
                "source": f"127.0.0.1:{source_port}",
                "endpoint": f"127.0.0.1:{19000 + i}",
                "tunnel_type": 0,
                "sender": 0,
                "receiver": 0,
                "description": desc,
                "enabled": 1,
                "username": "",
                "password": "",
                "is_compressed": 0,
                "encryption_method": "None",
                "custom_mapping": {},
            })
            if result.get("code") not in (0, 2):
                raise RuntimeError(f"add_tunnel {desc} failed: {result}")

        # Typed tunnels for filter tests.
        fixtures = [
            ("udp-alpha", 1, 1, 0, 0, 18100, 19100),
            ("socks-alice-sender", 2, 1, alice_id, 0, 18101, 0),
            ("http-bob-sender", 3, 1, bob_id, 0, 18102, 0),
            ("tcp-disabled", 0, 0, 0, 0, 18103, 19103),
            ("tcp-offline-bob-recv", 0, 1, 0, bob_id, 18104, 19104),
            ("tcp-alice-recv-only", 0, 1, 0, alice_id, 18105, 19105),
            ("tcp-secret-auth", 2, 1, alice_id, 0, 18106, 0),
        ]
        for desc, typ, enabled, sender, receiver, sport, eport in fixtures:
            payload = {
                "source": f"127.0.0.1:{sport}",
                "endpoint": f"127.0.0.1:{eport}" if typ in (0, 1) else "",
                "tunnel_type": typ,
                "sender": sender,
                "receiver": receiver,
                "description": desc,
                "enabled": enabled,
                "username": "proxy-user" if desc == "tcp-secret-auth" else "",
                "password": "s3cret-password" if desc == "tcp-secret-auth" else "",
                "is_compressed": 0,
                "encryption_method": "None",
                "custom_mapping": {},
            }
            result = cls._post_admin("/api/add_tunnel", payload)
            if result.get("code") not in (0, 2):
                raise RuntimeError(f"add_tunnel {desc} failed: {result}")

        time.sleep(0.8)
        listed = cls._post_admin("/api/tunnel_list", {"page_number": 0, "page_size": 100})
        for item in listed.get("tunnels", []):
            cls.tunnels[item.get("description") or f"id-{item['id']}"] = item

    def player_list(self, client: ApiClient | None = None, **kwargs) -> dict:
        api = client or self.admin
        return api.post("/api/player_list", kwargs)

    def tunnel_list(self, client: ApiClient | None = None, **kwargs) -> dict:
        api = client or self.admin
        return api.post("/api/tunnel_list", kwargs)

    def test_old_request_returns_query_version(self) -> None:
        players = self.player_list(page_number=0, page_size=20)
        tunnels = self.tunnel_list(page_number=0, page_size=20)
        self.assertEqual(players.get("query_version"), LIST_QUERY_VERSION)
        self.assertEqual(tunnels.get("query_version"), LIST_QUERY_VERSION)
        self.assertIn("players", players)
        self.assertIn("tunnels", tunnels)
        self.assertGreaterEqual(players["total_count"], 26)
        self.assertEqual(len(players["players"]), 20)
        self.assertGreaterEqual(tunnels["total_count"], 32)
        self.assertEqual(len(tunnels["tunnels"]), 20)

    def test_player_pagination_across_twenty(self) -> None:
        first = self.player_list(page_number=0, page_size=20, q="user")
        second = self.player_list(page_number=1, page_size=20, q="user")
        empty = self.player_list(page_number=2, page_size=20, q="user")
        self.assertEqual(first["total_count"], 25)
        self.assertEqual(len(first["players"]), 20)
        self.assertEqual(second["total_count"], 25)
        self.assertEqual(len(second["players"]), 5)
        self.assertEqual(empty["total_count"], 25)
        self.assertEqual(empty["players"], [])
        first_ids = [p["id"] for p in first["players"]]
        second_ids = [p["id"] for p in second["players"]]
        self.assertEqual(len(set(first_ids) & set(second_ids)), 0)
        self.assertEqual(first_ids, sorted(first_ids))
        self.assertEqual(second_ids, sorted(second_ids))

    def test_page_size_default_and_cap_and_overflow(self) -> None:
        defaulted = self.player_list(page_number=0, page_size=0, q="user")
        self.assertEqual(len(defaulted["players"]), 20)
        capped = self.player_list(page_number=0, page_size=1000)
        self.assertLessEqual(len(capped["players"]), 100)
        overflow = self.player_list(page_number=10**12, page_size=20)
        self.assertEqual(overflow["players"], [])
        self.assertGreaterEqual(overflow["total_count"], 26)
        self.assertEqual(overflow["query_version"], LIST_QUERY_VERSION)

    def test_player_search_and_special_characters(self) -> None:
        case = self.player_list(page_number=0, page_size=20, q="UsEr00")
        self.assertTrue(any(p["username"] == "user00" for p in case["players"]))
        special = self.player_list(page_number=0, page_size=20, q="literal%_name")
        self.assertEqual(special["total_count"], 1)
        self.assertEqual(special["players"][0]["username"], "literal%_name")
        wildcard = self.player_list(page_number=0, page_size=100, q="%")
        self.assertEqual(wildcard["total_count"], 1)
        self.assertEqual(wildcard["players"][0]["username"], "literal%_name")
        target = self.users["user07"]
        by_id = self.player_list(page_number=0, page_size=20, q=str(target["id"]))
        self.assertTrue(any(p["id"] == target["id"] for p in by_id["players"]))

    def test_player_combination_and_online_filters(self) -> None:
        enabled = self.player_list(page_number=0, page_size=100, enabled=1, q="user")
        disabled = self.player_list(page_number=0, page_size=100, enabled=0, q="user")
        self.assertEqual(enabled["total_count"], 20)
        self.assertEqual(disabled["total_count"], 5)
        self.assertTrue(all(p["enabled"] for p in enabled["players"]))
        self.assertTrue(all(not p["enabled"] for p in disabled["players"]))
        combo = self.player_list(page_number=0, page_size=100, enabled=1, web_access=1)
        names = {p["username"] for p in combo["players"]}
        self.assertEqual(names, {"user00", "user01"})
        online = self.player_list(page_number=0, page_size=100, online=1)
        offline = self.player_list(page_number=0, page_size=100, online=0)
        self.assertEqual(online["total_count"], 0)
        self.assertEqual(offline["total_count"], enabled["total_count"] + disabled["total_count"] + 1)
        self.assertTrue(all(not p["online"] for p in offline["players"]))

    def test_player_invalid_params(self) -> None:
        bad_flag = self.player_list(page_number=0, page_size=20, enabled=2)
        self.assertEqual(bad_flag.get("code"), -1)
        too_long = self.player_list(page_number=0, page_size=20, q="a" * 65)
        self.assertEqual(too_long.get("code"), -1)

    def test_tunnel_pagination_and_stable_sort(self) -> None:
        first = self.tunnel_list(page_number=0, page_size=20, q="tcp-bulk")
        second = self.tunnel_list(page_number=1, page_size=20, q="tcp-bulk")
        empty = self.tunnel_list(page_number=2, page_size=20, q="tcp-bulk")
        self.assertEqual(first["total_count"], 25)
        self.assertEqual(len(first["tunnels"]), 20)
        self.assertEqual(len(second["tunnels"]), 5)
        self.assertEqual(empty["tunnels"], [])
        ids = [t["id"] for t in first["tunnels"] + second["tunnels"]]
        self.assertEqual(ids, sorted(ids))
        self.assertEqual(len(ids), len(set(ids)))

    def test_tunnel_type_enabled_and_listen_status(self) -> None:
        udp = self.tunnel_list(page_number=0, page_size=20, tunnel_type=1)
        self.assertGreaterEqual(udp["total_count"], 1)
        self.assertTrue(all(t["tunnel_type"] == 1 for t in udp["tunnels"]))
        http = self.tunnel_list(page_number=0, page_size=20, tunnel_type=3)
        self.assertTrue(any(t["description"] == "http-bob-sender" for t in http["tunnels"]))
        disabled = self.tunnel_list(page_number=0, page_size=20, listen_status="disabled")
        self.assertTrue(all((t.get("listen_status") == "disabled") for t in disabled["tunnels"]))
        self.assertTrue(any(t["description"] == "tcp-disabled" for t in disabled["tunnels"]))
        offline = self.tunnel_list(page_number=0, page_size=20, listen_status="offline")
        self.assertTrue(any(t["description"] == "tcp-offline-bob-recv" for t in offline["tunnels"]))
        listening = self.tunnel_list(page_number=0, page_size=100, listen_status="listening")
        self.assertGreaterEqual(listening["total_count"], 1)
        self.assertTrue(all(t.get("listen_status") == "listening" for t in listening["tunnels"]))
        combo = self.tunnel_list(page_number=0, page_size=20, q="tcp-bulk", enabled=1, tunnel_type=0)
        self.assertEqual(combo["total_count"], 25)

    def test_tunnel_q_skips_password_and_proxy_username(self) -> None:
        secret = self.tunnel_list(page_number=0, page_size=20, q="s3cret-password")
        proxy = self.tunnel_list(page_number=0, page_size=20, q="proxy-user")
        self.assertEqual(secret["total_count"], 0)
        self.assertEqual(proxy["total_count"], 0)
        named = self.tunnel_list(page_number=0, page_size=20, q="tcp-secret-auth")
        self.assertEqual(named["total_count"], 1)
        # List items must not expose a password field.
        self.assertNotIn("password", named["tunnels"][0])

    def test_tunnel_user_id_and_role(self) -> None:
        alice = self.users["user00"]["id"]
        any_role = self.tunnel_list(page_number=0, page_size=100, user_id=alice, user_role="any")
        sender = self.tunnel_list(page_number=0, page_size=100, user_id=alice, user_role="sender")
        receiver = self.tunnel_list(page_number=0, page_size=100, user_id=alice, user_role="receiver")
        self.assertGreaterEqual(any_role["total_count"], 2)
        self.assertTrue(all(t["sender"] == alice or t["receiver"] == alice for t in any_role["tunnels"]))
        self.assertTrue(all(t["sender"] == alice for t in sender["tunnels"]))
        self.assertTrue(all(t["receiver"] == alice for t in receiver["tunnels"]))
        missing_id = self.tunnel_list(page_number=0, page_size=20, user_role="sender")
        self.assertEqual(missing_id.get("code"), -1)
        bad_type = self.tunnel_list(page_number=0, page_size=20, tunnel_type=4)
        self.assertEqual(bad_type.get("code"), -1)
        bad_status = self.tunnel_list(page_number=0, page_size=20, listen_status="up")
        self.assertEqual(bad_status.get("code"), -1)

    def test_regular_user_scope_and_forbidden_user_id(self) -> None:
        alice_name = "user00"
        client = ApiClient(self.server.base_url, alice_name, self.passwords[alice_name])
        client.login()
        players = self.player_list(client, page_number=0, page_size=20)
        self.assertEqual(players["total_count"], 1)
        self.assertEqual(players["players"][0]["username"], alice_name)
        other = self.player_list(client, page_number=0, page_size=20, q="user01")
        self.assertEqual(other["total_count"], 0)
        bob = self.users["user01"]["id"]
        forbidden = self.tunnel_list(client, page_number=0, page_size=20, user_id=bob, user_role="any")
        self.assertEqual(forbidden.get("code"), 403)
        own = self.tunnel_list(
            client,
            page_number=0,
            page_size=100,
            user_id=self.users[alice_name]["id"],
            user_role="any",
        )
        self.assertEqual(own.get("query_version"), LIST_QUERY_VERSION)
        self.assertTrue(all(
            t["sender"] == self.users[alice_name]["id"] or t["receiver"] == self.users[alice_name]["id"]
            for t in own["tunnels"]
        ))

    def test_receiver_only_cannot_read_password(self) -> None:
        alice_name = "user00"
        client = ApiClient(self.server.base_url, alice_name, self.passwords[alice_name])
        client.login()
        recv_only = self.tunnels["tcp-alice-recv-only"]
        detail = client.post("/api/tunnel_detail", {"id": recv_only["id"]})
        self.assertEqual(detail.get("code"), 403)
        self.assertIsNone(detail.get("tunnel"))
        # Visible in the list, but still no password field.
        listed = self.tunnel_list(client, page_number=0, page_size=100, q="tcp-alice-recv-only")
        self.assertEqual(listed["total_count"], 1)
        self.assertNotIn("password", listed["tunnels"][0])

    def test_frontend_old_server_degrade_contract(self) -> None:
        """Documents the contract the frontend uses for graceful fallback."""
        modern = self.tunnel_list(page_number=0, page_size=1, user_id=self.users["user00"]["id"], user_role="any")
        self.assertEqual(modern.get("query_version"), LIST_QUERY_VERSION)
        # A payload without query_version must not be treated as a user-filtered list.
        stale = {"tunnels": modern["tunnels"], "total_count": modern["total_count"]}
        self.assertIsNone(stale.get("query_version"))


def _maybe_test_online_with_client(suite_result_factory=None) -> None:
    # Optional extra check is included in the class when a client is online=0 only.
    # Connecting np_client is best-effort and covered separately if the binary exists.
    pass


def main() -> int:
    parser = argparse.ArgumentParser(description="Isolated player_list/tunnel_list API tests")
    parser.add_argument("--build", action="store_true", help="Build np_server if the debug binary is missing")
    parser.add_argument("--keep-temp", action="store_true")
    args, rest = parser.parse_known_args()
    if args.build:
        os.environ["NPIPE_TEST_BUILD"] = "1"
        ensure_binary(binary_path("np_server"), "np_server", True)
    IsolatedServer.keep_temp_default = args.keep_temp  # type: ignore[attr-defined]
    argv = [sys.argv[0], *rest]
    result = unittest.main(argv=argv, exit=False, verbosity=2)
    return 0 if result.result.wasSuccessful() else 1


if __name__ == "__main__":
    sys.exit(main())
