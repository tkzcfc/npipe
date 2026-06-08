"""
npipe 管理员 REST API 客户端
"""
import logging
import requests
from typing import Optional, Dict, Any

logger = logging.getLogger("admin_api")


class AdminAPIError(Exception):
    pass


class AdminAPI:
    """封装 npipe 服务端 Web API"""

    def __init__(self, base_url: str, username: str, password: str):
        self.base_url = base_url.rstrip("/")
        self.username = username
        self.password = password
        self._session = requests.Session()
        self._logged_in = False

    # ------------------------------------------------------------------
    # 认证
    # ------------------------------------------------------------------
    def login(self) -> bool:
        url = f"{self.base_url}/api/login"
        try:
            resp = self._session.post(
                url,
                json={"username": self.username, "password": self.password},
                timeout=10,
            )
            data = resp.json()
            if data.get("code") == 0:
                self._logged_in = True
                logger.info("Admin API login success")
                return True
            else:
                logger.error(f"Admin API login failed: {data.get('msg')}")
                return False
        except Exception as e:
            logger.error(f"Admin API login error: {e}")
            return False

    def logout(self):
        try:
            self._session.post(f"{self.base_url}/api/logout", timeout=5)
        except Exception:
            pass
        self._logged_in = False

    def _ensure_login(self):
        if not self._logged_in:
            if not self.login():
                raise AdminAPIError("Cannot login to admin API")

    def _post(self, path: str, data: Dict[str, Any]) -> Dict[str, Any]:
        self._ensure_login()
        url = f"{self.base_url}{path}"
        resp = self._session.post(url, json=data, timeout=10)
        resp.raise_for_status()
        return resp.json()

    # ------------------------------------------------------------------
    # 隧道 CRUD
    # ------------------------------------------------------------------
    def list_tunnels(self, page_number: int = 0, page_size: int = 100) -> list:
        result = self._post(
            "/api/tunnel_list",
            {"page_number": page_number, "page_size": page_size},
        )
        return result.get("tunnels", [])

    def add_tunnel(
        self,
        source: str,
        endpoint: str,
        tunnel_type: int,
        sender: int,
        receiver: int,
        description: str = "",
        enabled: int = 1,
        username: str = "",
        password: str = "",
        is_compressed: int = 0,
        encryption_method: str = "",
        custom_mapping: Optional[Dict[str, str]] = None,
    ) -> Dict[str, Any]:
        result = self._post(
            "/api/add_tunnel",
            {
                "source": source,
                "endpoint": endpoint,
                "tunnel_type": tunnel_type,
                "sender": sender,
                "receiver": receiver,
                "description": description,
                "enabled": enabled,
                "username": username,
                "password": password,
                "is_compressed": is_compressed,
                "encryption_method": encryption_method,
                "custom_mapping": custom_mapping or {},
            },
        )
        return result

    def remove_tunnel(self, tunnel_id: int) -> Dict[str, Any]:
        return self._post("/api/remove_tunnel", {"id": tunnel_id})

    def find_tunnel_by_description(self, description: str) -> Optional[Dict]:
        tunnels = self.list_tunnels()
        for t in tunnels:
            if t.get("description") == description:
                return t
        return None

    # ------------------------------------------------------------------
    # 玩家
    # ------------------------------------------------------------------
    def list_players(self, page_number: int = 0, page_size: int = 100) -> list:
        result = self._post(
            "/api/player_list",
            {"page_number": page_number, "page_size": page_size},
        )
        return result.get("players", [])

    def find_player_by_username(self, username: str) -> Optional[Dict]:
        players = self.list_players()
        for player in players:
            if player.get("username") == username:
                return player
        return None

    def add_player(self, username: str, password: str) -> Dict[str, Any]:
        return self._post(
            "/api/add_player",
            {"username": username, "password": password},
        )
