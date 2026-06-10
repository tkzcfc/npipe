# npipe

[🇬🇧 English](./README.md)

**npipe** 是一个用 Rust 编写的跨平台（Windows / Linux / macOS）安全隧道与代理工具。它通过加密隧道将多路 TCP/UDP 流量安全地转发到远端主机，并提供丰富的代理协议支持和 Web 管理后台。

---

## ✨ 功能特性

- **多协议代理**
  - TCP / UDP 端口转发（本地 & 远端）
  - SOCKS5 代理服务器（本地 & 远端）
  - HTTP 代理服务器（本地 & 远端）
- **多传输协议**（可按需编译）
  - `tcp`  — 标准 TCP
  - `kcp`  — 低延迟 KCP
  - `ws`   — WebSocket（穿透限制）
  - `quic` — QUIC / HTTP3（快速握手）
- **安全**
  - TLS 加密传输（可选）
  - 代理通道独立加密（Xor 等）+ LZ4 压缩
- **Web 管理后台**（actix-web + Vue.js）
  - 用户/隧道管理
  - 实时在线状态监控
- **Windows 服务**：客户端可注册为系统服务，开机自启
- **非法流量转发**：将非 npipe 流量透明转发至其他程序（如 Nginx）
- **多数据库**：SQLite（默认）/ MySQL

---

## 📦 项目结构

```
npipe/
├── np_base/      # 核心基础库（网络、代理、加密）
├── np_proto/     # 协议定义（Protobuf）
├── np_server/    # 服务端
├── np_client/    # 客户端
└── npipe-admin/  # Web 管理前端（Vue 3）
```

---

## 🚀 快速开始

### 编译

确保已安装 [Rust 工具链](https://rustup.rs/)（推荐 stable 版本）。

```bash
# 编译所有组件（含 KCP / WebSocket / QUIC）
cargo build --release

# 仅编译 TCP 支持（最小体积）
cargo build --release --no-default-features --features tcp
```

> **跨平台编译**：项目提供了 `Cross.toml`，可使用 [cross](https://github.com/cross-rs/cross) 进行跨架构编译。

---

## ⚙️ 服务端

### 配置文件

默认读取同目录下的 `config.json`，可使用 `-c` 指定其他路径。

```json
{
    "database_url": "sqlite://data.db?mode=rwc",
    "listen_addr": "tcp://0.0.0.0:8118,kcp://0.0.0.0:8118,ws://0.0.0.0:8119,quic://0.0.0.0:8119",
    "illegal_traffic_forward": "",
    "illegal_traffic_forward_rules": [
        {
            "name": "http-to-nginx",
            "match_expr": "http",
            "target": "127.0.0.1:80"
        },
        {
            "name": "tls-to-https",
            "match_expr": "tls",
            "target": "127.0.0.1:443"
        }
    ],
    "enable_tls": false,
    "tls_cert": "./cert.pem",
    "tls_key": "./server.key.pem",
    "web_base_dir": "./dist",
    "web_addr": "0.0.0.0:8120",
    "web_enable_tls": false,
    "web_tls_cert": "./web-cert.pem",
    "web_tls_key": "./web-key.pem",
    "web_tls_auto_self_signed": false,
    "web_cookie_secure": false,
    "web_username": "admin",
    "web_password": "admin@1234",
    "transport_max_connections_per_player": 16,
    "transport_idle_timeout_secs": 60,
    "quiet": false,
    "log_dir": "logs"
}
```

#### 配置项说明

| 配置项                    | 说明                                                                 | 示例                                                                |
|---------------------------|----------------------------------------------------------------------|---------------------------------------------------------------------|
| `database_url`            | 数据库连接地址                                                       | `sqlite://data.db?mode=rwc`<br>`mysql://user:pass@host:3306/dbname` |
| `listen_addr`             | 服务监听地址，多个地址用逗号分隔                                     | `tcp://0.0.0.0:8118,kcp://0.0.0.0:8118,ws://0.0.0.0:8119`         |
| `enable_tls`              | 是否启用 TLS                                                         | `true` / `false`                                                    |
| `tls_cert`                | TLS 证书文件路径                                                     | `./cert.pem`                                                        |
| `tls_key`                 | TLS 私钥文件路径                                                     | `./server.key.pem`                                                  |
| `web_base_dir`            | Web 管理前端静态文件目录（留空则禁用 Web 管理）                      | `./dist`                                                            |
| `web_addr`                | Web 管理后台监听地址                                                 | `0.0.0.0:8120`                                                      |
| `web_enable_tls`          | 是否为 Web 管理后台直接启用 HTTPS                                    | `true` / `false`                                                    |
| `web_tls_cert`            | Web 管理后台 HTTPS 证书文件路径                                      | `./web-cert.pem`                                                    |
| `web_tls_key`             | Web 管理后台 HTTPS 私钥文件路径                                      | `./web-key.pem`                                                     |
| `web_tls_auto_self_signed`| Web 管理后台 HTTPS 证书为空时，是否自动生成临时自签名证书            | `true` / `false`                                                    |
| `web_cookie_secure`       | 是否强制 Web 管理后台 Session Cookie 使用 Secure；外部 Nginx/HTTPS 反代时建议设为 `true` | `true` / `false`                                                    |
| `web_username`            | Web 管理员账号（留空则禁用 Web 管理）                                | `admin`                                                             |
| `web_password`            | Web 管理员密码（留空则禁用 Web 管理）                                | `admin@1234`                                                        |
| `transport_max_connections_per_player` | 每个用户允许的最大转发连接/流数量，`0` 表示保持单连接模式 | `0` / `4` / `8`                                                     |
| `transport_idle_timeout_secs` | 转发连接/流空闲关闭时间（秒），`0` 表示不因空闲主动关闭              | `60`                                                                |
| `illegal_traffic_forward` | 非 npipe 流量转发地址，可对接 Nginx 等（留空则丢弃）                 | `127.0.0.1:80`                                                      |
| `illegal_traffic_forward_rules` | 非法流量转发规则数组，支持按流量类型匹配转发（见下方详细说明） | 见示例                                                               |
| `quiet`                   | 安静模式，不输出日志                                                 | `true` / `false`                                                    |
| `log_dir`                 | 日志保存目录                                                         | `logs`                                                              |

#### ⚠️ 注意事项

- **QUIC 协议必须启用 TLS**：QUIC 协议在设计上强制要求加密，若 `listen_addr` 中包含 `quic://` 地址，必须同时将 `enable_tls` 设为 `true` 并提供有效的 `tls_cert` 和 `tls_key`，否则服务端将无法正常启动 QUIC 监听。
- **传输多连接**：服务端 `transport_max_connections_per_player` 默认为 `16`，客户端 `--transport-max-connections` 默认为 `16`。任意一侧为 `0` 时都会保持单连接模式；两侧都大于 `0` 时，最终数量取较小值。客户端 `--transport-min-connections`（默认 `4`）控制登录后预热的最小连接数，空闲回收不会低于此值。QUIC 使用同一条 QUIC 连接上的多 stream，TCP / KCP / WebSocket 使用多条转发连接。
- **多协议混用**：TCP / KCP / WebSocket 可以在不启用 TLS 的情况下运行，但建议生产环境统一开启 TLS 以保障传输安全。
- **Web 管理 HTTPS**：`web_enable_tls` 只控制 Web 管理后台是否直接启用 HTTPS，和隧道服务的 `enable_tls` 相互独立；启用时必须配置 `web_tls_cert` 和 `web_tls_key`。
- **临时自签名证书**：如果 `web_enable_tls` 为 `true` 且未配置 `web_tls_cert` / `web_tls_key`，可将 `web_tls_auto_self_signed` 设为 `true` 自动生成临时自签名证书；浏览器会提示证书不受信任，仅建议临时测试使用。
- **HTTPS 反向代理**：如果浏览器通过 Nginx 等 HTTPS 代理访问后台，而 `np_server` 到代理之间是 HTTP，请将 `web_cookie_secure` 设为 `true`，让后台 Session Cookie 只通过 HTTPS 发送。
- **Web 管理禁用**：`web_username`、`web_password`、`web_addr` 三者任意一项为空，Web 管理后台将自动关闭。

#### 非法流量转发规则 (`illegal_traffic_forward_rules`)

当服务端收到非 npipe 协议的流量时，可通过 `illegal_traffic_forward` 简单转发到单一地址，也可通过 `illegal_traffic_forward_rules` 按流量类型精确匹配后分发到不同目标。

每条规则包含三个字段：

| 字段         | 说明                           |
|--------------|--------------------------------|
| `name`       | 规则名称（仅标识用途）         |
| `match_expr` | 匹配表达式（见下方语法）       |
| `target`     | 匹配后转发到的目标地址         |

**`match_expr` 语法**：

| 表达式              | 说明                                           | 示例                       |
|---------------------|------------------------------------------------|----------------------------|
| `http`              | 检测 HTTP 方法开头（GET/POST/PUT/DELETE 等）   | `"http"`                   |
| `tls`               | 检测 TLS ClientHello（`0x16 0x03` 开头）       | `"tls"`                    |
| `any`               | 兜底，永远匹配                                 | `"any"`                    |
| `prefix:<hex>`      | 十六进制字节前缀匹配                           | `"prefix:1603"`（即 TLS）  |
| `prefix:str:<s>`    | 字符串前缀匹配                                 | `"prefix:str:GET "`        |
| `regex:<pattern>`   | 正则表达式匹配                                 | `"regex:^(GET|POST) "`     |

> **注意**：规则按数组顺序依次匹配，命中即停止。若同时配置了 `illegal_traffic_forward`，它等价于一条 `match_expr` 为 `any` 的兜底规则，放在所有 `illegal_traffic_forward_rules` 之后执行。

**配置示例**：

```json
{
    "illegal_traffic_forward_rules": [
        {
            "name": "http-to-nginx",
            "match_expr": "http",
            "target": "127.0.0.1:80"
        },
        {
            "name": "tls-to-https",
            "match_expr": "tls",
            "target": "127.0.0.1:443"
        },
        {
            "name": "rdp",
            "match_expr": "prefix:0300",
            "target": "192.168.1.100:3389"
        }
    ]
}
```

### 启动服务端

```bash
# 使用默认配置文件
np_server

# 指定配置文件
np_server -c /etc/npipe/config.json

# 查看帮助
np_server --help
```

```
Usage: np_server [OPTIONS]

Options:
  -b, --backtrace <BACKTRACE>              打印回溯信息 [default: false]
  -c, --config-file <CONFIG_FILE>          配置文件路径 [default: config.json]
      --log-level <LOG_LEVEL>              日志级别 [default: info]
      --base-log-level <BASE_LOG_LEVEL>    基础库日志级别 [default: error]
  -h, --help                               打印帮助
  -V, --version                            打印版本
```

---

## 🖥️ 客户端

### 运行客户端

```bash
np_client run --server tcp://your-server:8118 --username user1 --password pass123
```

支持同时指定多个服务端地址，客户端将循环尝试连接：

```bash
np_client run \
  --server "tcp://server1:8118,kcp://server2:8118" \
  --username user1 \
  --password pass123 \
  --transport-max-connections 4 \
  --transport-min-connections 2 \
  --transport-idle-timeout-secs 60 \
  --enable-tls \
  --ca-cert ./root-ca.pem
```

```
Usage: np_client run [OPTIONS] --server <SERVER> --username <USERNAME> --password <PASSWORD>

Options:
  -s, --server <SERVER>                    服务器地址（多个地址用逗号分隔，循环重连）
  -u, --username <USERNAME>                用户名
  -p, --password <PASSWORD>                密码
      --enable-tls                         启用 TLS
      --tls-server-name <NAME>             TLS SNI 服务器名（可选）
      --insecure                           不验证服务器证书（不推荐生产使用）
      --ca-cert <CA_CERT>                  CA 证书文件路径
      --transport-max-connections <N>      最大转发连接/流数量，0 保持单连接模式 [default: 16]
      --transport-min-connections <N>      最小保持连接数（预热），0 禁用预热 [default: 4]
      --transport-idle-timeout-secs <SECS> 转发连接/流空闲关闭时间（秒） [default: 60]
      --log-level <LOG_LEVEL>              日志级别 [default: info]
      --base-log-level <BASE_LOG_LEVEL>    基础库日志级别 [default: error]
      --log-dir <LOG_DIR>                  日志目录 [default: logs]
      --quiet                              安静模式，不输出日志
      --backtrace <BACKTRACE>              打印回溯信息 [default: false]
  -h, --help                               打印帮助
```

### Windows 服务（仅 Windows）

以管理员权限在命令提示符中执行以下命令可将客户端注册为 Windows 系统服务：

```bat
:: 安装服务
np_client.exe install --server tcp://your-server:8118 --username user1 --password pass123

:: 启动服务
sc.exe start "np_client"

:: 停止服务
sc.exe stop "np_client"

:: 卸载服务
np_client.exe uninstall
```

---

## 🔐 TLS 证书生成

项目内置了证书生成脚本，一键生成自签名 CA 证书及服务端证书：

```bash
./generate-certificate.sh
```

生成后会在当前目录产生以下文件：

| 文件               | 用途                      |
|--------------------|---------------------------|
| `root-ca.pem`      | 根 CA 证书（客户端信任）   |
| `root-ca.key.pem`  | 根 CA 私钥                |
| `cert.pem`         | 服务端证书                |
| `server.key.pem`   | 服务端私钥                |

---

## 🌐 Web 管理后台

Web 管理后台基于 **Vue 3 + Vite** 构建，源码位于 `npipe-admin/`。

```bash
# 安装依赖
cd npipe-admin
npm install

# 开发模式
npm run dev

# 构建生产版本（产物输出至 dist/）
npm run build
```

将构建产物目录配置到服务端 `web_base_dir` 后，访问 `http://<server-ip>:<web_port>` 即可打开管理界面。

---


## 📄 许可证

本项目基于 [LICENSE](./LICENSE) 协议开源。

---

## 🔗 相关链接

- 项目主页：[https://github.com/tkzcfc/npipe](https://github.com/tkzcfc/npipe)
