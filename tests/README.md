# npipe 代理工具测试套件

用于对 npipe 内网穿透工具进行端到端功能验证，支持四种隧道类型：

| 类型     | 说明                                     |
|--------|----------------------------------------|
| TCP    | 端口转发，原始 TCP 数据透传                       |
| UDP    | UDP 数据包转发                              |
| SOCKS5 | SOCKS5 代理协议（支持无认证 / 用户名密码认证，TCP+UDP） |
| HTTP   | HTTP 代理（CONNECT 隧道）                    |

---

## 环境要求

- Python 3.10+
- pip 安装依赖：

```bash
pip install -r requirements.txt
```

---

## 快速开始

### 0. 传输协商冒烟测试

这个测试会自动启动一个临时 `np_server`，使用临时 SQLite 数据库，通过 Admin API 创建测试用户，然后启动 `np_client` 验证登录和传输协商日志。它不会使用或修改项目根目录的 `data.db`。

```bash
# 在 npipe 项目根目录
cargo build -p np_server -p np_client
python tests/transport_smoke.py
```

如果还没有 debug 二进制，可以让脚本自动构建：

```bash
python tests/transport_smoke.py --build
```

### 1. 确保 npipe 服务端已启动

```bash
# 在 npipe 项目根目录
cargo run --bin np_server
```

### 2. 确保至少一个 np_client 已连接

客户端需登录到服务端，兼作 sender（入口）和 receiver（出口）。

单连接模式：

```bash
cargo run --bin np_client -- run \
  --server tcp://127.0.0.1:8118 \
  --username user1 \
  --password pass123 \
  --transport-max-connections 0
```

多连接/多流模式：

```bash
cargo run --bin np_client -- run \
  --server tcp://127.0.0.1:8118 \
  --username user1 \
  --password pass123 \
  --transport-max-connections 4 \
  --transport-idle-timeout-secs 60
```

> 服务端 `transport_max_connections_per_player` 和客户端 `--transport-max-connections` 任意一侧为 `0` 时都会保持单连接模式；两侧都大于 `0` 时最终使用较小值。QUIC 使用同一条 QUIC 连接上的多 stream，TCP/KCP/WebSocket 使用多条转发连接。

### 3. 根据实际情况修改 `test_config.json`

重点字段：

| 字段                         | 说明                                    |
|----------------------------|---------------------------------------|
| `admin.url`                | np_server 的 Web API 地址                |
| `admin.username/password`  | 管理员账号（对应 config.json 中的 web_username/web_password） |
| `players.sender_id`        | 入口端客户端的玩家 ID（在管理界面可查）               |
| `players.receiver_id`      | 出口端客户端的玩家 ID                         |
| `tunnels.*.inlet_port`     | 各类型隧道的本地监听端口                         |
| `test.create_tunnels`      | `true` = 自动通过 API 创建测试隧道               |
| `test.cleanup_after_test`  | `true` = 测试后自动删除创建的隧道                 |

### 4. 运行测试

```bash
# 在 tests/ 目录下
cd tests

# 运行全部测试
python proxy_tester.py

# 只测试 TCP 和 SOCKS5
python proxy_tester.py --type tcp socks5

# 使用已有隧道（不自动创建）
python proxy_tester.py --no-create

# 测试后不清理隧道
python proxy_tester.py --no-cleanup

# 指定配置文件
python proxy_tester.py -c /path/to/config.json

# 详细日志
python proxy_tester.py -v
```

---

## 测试原理

```
测试脚本
  ├── 启动本地 TCP Echo Server（19001 端口）
  ├── 启动本地 UDP Echo Server（19002 端口）
  ├── 通过 Admin API 创建测试隧道
  │
  ├── TCP 测试
  │     客户端 → inlet(19101) → [npipe隧道] → outlet → Echo(19001)
  │     验证：发送随机数据，收到相同数据
  │
  ├── UDP 测试
  │     客户端 → inlet(19102) → [npipe隧道] → outlet → Echo(19002)
  │     验证：发送随机数据包，收到相同数据包
  │
  ├── SOCKS5 测试
  │     客户端 → SOCKS5握手 → inlet(19103) → [npipe隧道] → outlet → Echo(19001)
  │     验证：完整 SOCKS5 协商 + 数据回显
  │
  └── HTTP 代理测试
        客户端 → CONNECT请求 → inlet(19104) → [npipe隧道] → outlet → Echo(19001)
        验证：HTTP/1.1 CONNECT 200 + 数据回显
```

---

## 文件说明

| 文件                | 作用                        |
|-------------------|---------------------------|
| `proxy_tester.py` | 主入口，CLI 参数处理 + 测试编排        |
| `test_config.json`| 测试配置（端口、账号、超时等）           |
| `echo_server.py`  | 本地 TCP/UDP 回显服务器           |
| `admin_api.py`    | npipe Admin REST API 客户端   |
| `transport_smoke.py` | 启动临时服务端/客户端，验证登录和传输协商 |
| `tester_tcp.py`   | TCP 隧道测试逻辑                 |
| `tester_udp.py`   | UDP 隧道测试逻辑                 |
| `tester_socks5.py`| SOCKS5 代理测试逻辑（手动实现协议握手）   |
| `tester_http.py`  | HTTP 代理测试逻辑（CONNECT + 普通代理）|
| `requirements.txt`| Python 依赖                  |

---

## 常见问题

**Q: `Admin API 连接失败`**  
A: 确认 np_server 已启动，`admin.url` 和账号密码与 `config.json` 中一致。

**Q: 隧道创建成功但测试超时**  
A: 入口在 np_client 一侧监听，需确认 `sender_id` 对应的客户端已连接并在线。可在 npipe-admin 管理界面查看玩家在线状态。

**Q: UDP 测试偶尔失败**  
A: UDP 本身不保证可靠，测试会自动重试 5 次。若持续失败，适当增大 `test.timeout`。

**Q: SOCKS5 认证失败**  
A: 检查 `tunnels.socks5.username/password` 是否与隧道配置一致（留空表示不认证）。
