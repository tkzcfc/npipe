# npipe

[🇨🇳 中文文档](./README_CN.md)

**npipe** is a cross-platform (Windows / Linux / macOS) secure tunneling and proxy tool written in Rust. It securely forwards multiplexed TCP/UDP traffic to remote hosts over encrypted tunnels, with rich proxy protocol support and a web management dashboard.

---

## ✨ Features

- **Multi-protocol Proxy**
  - TCP / UDP port forwarding (local & remote)
  - SOCKS5 proxy server (local & remote)
  - HTTP proxy server (local & remote)
- **Multiple Transports** (compile-time selectable)
  - `tcp`  — Standard TCP
  - `kcp`  — Low-latency KCP
  - `ws`   — WebSocket (firewall traversal)
  - `quic` — QUIC / HTTP3 (fast handshake)
- **Security**
  - TLS encrypted transport (optional)
  - Per-tunnel encryption (Xor, etc.) + LZ4 compression
- **Web Dashboard** (actix-web + Vue.js)
  - User & tunnel management
  - Real-time online status monitoring
- **Windows Service**: Client can be registered as a system service for auto-start
- **Non-npipe Traffic Forwarding**: Transparently forward non-npipe traffic to other programs (e.g., Nginx)
- **Multiple Databases**: SQLite (default) / MySQL

---

## 📦 Project Structure

```
npipe/
├── np_base/      # Core library (networking, proxy, encryption)
├── np_proto/     # Protocol definitions (Protobuf)
├── np_server/    # Server
├── np_client/    # Client
└── npipe-admin/  # Web admin frontend (Vue 3)
```

---

## 🚀 Quick Start

### Build

Ensure the [Rust toolchain](https://rustup.rs/) is installed (stable recommended).

```bash
# Build all components (with KCP / WebSocket / QUIC)
cargo build --release

# Build with TCP support only (minimal binary size)
cargo build --release --no-default-features --features tcp
```

> **Cross-compilation**: The project includes `Cross.toml` for use with [cross](https://github.com/cross-rs/cross).

---

## ⚙️ Server

### Configuration

The server reads `config.json` from the current directory by default. Use `-c` to specify an alternative path.

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

#### Configuration Reference

| Option                    | Description                                                          | Example                                                             |
|---------------------------|----------------------------------------------------------------------|---------------------------------------------------------------------|
| `database_url`            | Database connection URL                                              | `sqlite://data.db?mode=rwc`<br>`mysql://user:pass@host:3306/dbname` |
| `listen_addr`             | Server listen addresses, comma-separated                             | `tcp://0.0.0.0:8118,kcp://0.0.0.0:8118,ws://0.0.0.0:8119`         |
| `enable_tls`              | Enable TLS                                                           | `true` / `false`                                                    |
| `tls_cert`                | TLS certificate file path                                            | `./cert.pem`                                                        |
| `tls_key`                 | TLS private key file path                                            | `./server.key.pem`                                                  |
| `web_base_dir`            | Web frontend static files directory (empty to disable)               | `./dist`                                                            |
| `web_addr`                | Web dashboard listen address                                         | `0.0.0.0:8120`                                                      |
| `web_enable_tls`          | Enable HTTPS for the web dashboard directly                          | `true` / `false`                                                    |
| `web_tls_cert`            | Web dashboard HTTPS certificate path                                 | `./web-cert.pem`                                                    |
| `web_tls_key`             | Web dashboard HTTPS private key path                                 | `./web-key.pem`                                                     |
| `web_tls_auto_self_signed`| Auto-generate a temporary self-signed certificate when cert is empty | `true` / `false`                                                    |
| `web_cookie_secure`       | Force Secure flag on session cookies; recommended when behind HTTPS reverse proxy | `true` / `false`                                       |
| `web_username`            | Web admin username (empty to disable web dashboard)                  | `admin`                                                             |
| `web_password`            | Web admin password (empty to disable web dashboard)                  | `admin@1234`                                                        |
| `transport_max_connections_per_player` | Max forward connections/streams per user; `0` = single-connection mode | `0` / `4` / `8`                                              |
| `transport_idle_timeout_secs` | Forward connection idle timeout (seconds); `0` = never close     | `60`                                                                |
| `illegal_traffic_forward` | Forward non-npipe traffic to this address (empty to discard)         | `127.0.0.1:80`                                                      |
| `illegal_traffic_forward_rules` | Traffic forwarding rules array (see detailed explanation below) | See example                                                         |
| `quiet`                   | Quiet mode, suppress log output                                      | `true` / `false`                                                    |
| `log_dir`                 | Log output directory                                                 | `logs`                                                              |

#### ⚠️ Notes

- **QUIC requires TLS**: QUIC mandates encryption by design. If `listen_addr` contains a `quic://` address, `enable_tls` must be `true` with valid `tls_cert` and `tls_key`, otherwise the QUIC listener will fail to start.
- **Transport multi-connection**: Server `transport_max_connections_per_player` defaults to `16`; client `--transport-max-connections` defaults to `16`. If either side is `0`, single-connection mode is used; when both are > 0, the smaller value wins. Client `--transport-min-connections` (default `4`) controls the minimum pre-warmed connections after login; idle cleanup will not drop below this value. QUIC uses multiple streams on one QUIC connection; TCP / KCP / WebSocket use multiple forward connections.
- **Mixed protocols**: TCP / KCP / WebSocket can run without TLS, but enabling TLS is recommended in production.
- **Web dashboard HTTPS**: `web_enable_tls` only controls the web dashboard and is independent of the tunnel service's `enable_tls`; both `web_tls_cert` and `web_tls_key` must be configured when enabled.
- **Temporary self-signed certificate**: If `web_enable_tls` is `true` but `web_tls_cert` / `web_tls_key` are not configured, set `web_tls_auto_self_signed` to `true` to auto-generate a temporary self-signed certificate. Browsers will show an untrusted certificate warning; recommended only for testing.
- **HTTPS reverse proxy**: If the browser accesses the dashboard via an HTTPS proxy (e.g., Nginx) while `np_server` communicates with the proxy over HTTP, set `web_cookie_secure` to `true` so session cookies are only sent over HTTPS.
- **Disabling web dashboard**: If any of `web_username`, `web_password`, or `web_addr` is empty, the web dashboard is automatically disabled.

#### Non-npipe Traffic Forwarding Rules (`illegal_traffic_forward_rules`)

When the server receives non-npipe protocol traffic, it can forward to a single address via `illegal_traffic_forward`, or use `illegal_traffic_forward_rules` to match and dispatch to different targets by traffic type.

Each rule has three fields:

| Field        | Description                        |
|--------------|------------------------------------|
| `name`       | Rule name (for identification)     |
| `match_expr` | Match expression (see syntax below)|
| `target`     | Target address to forward to       |

**`match_expr` syntax**:

| Expression          | Description                                        | Example                    |
|---------------------|----------------------------------------------------|----------------------------|
| `http`              | Detect HTTP method prefix (GET/POST/PUT/DELETE etc)| `"http"`                   |
| `tls`              | Detect TLS ClientHello (`0x16 0x03` prefix)        | `"tls"`                    |
| `any`               | Catch-all, always matches                          | `"any"`                    |
| `prefix:<hex>`      | Hex byte prefix match                              | `"prefix:1603"` (i.e. TLS)|
| `prefix:str:<s>`    | String prefix match                                | `"prefix:str:GET "`        |
| `regex:<pattern>`   | Regular expression match                           | `"regex:^(GET|POST) "`     |

> **Note**: Rules are matched in array order; first match wins. If `illegal_traffic_forward` is also configured, it acts as a catch-all (`any`) rule appended after all `illegal_traffic_forward_rules`.

**Example configuration**:

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

### Starting the Server

```bash
# Use default config file
np_server

# Specify config file
np_server -c /etc/npipe/config.json

# Show help
np_server --help
```

```
Usage: np_server [OPTIONS]

Options:
  -b, --backtrace <BACKTRACE>              Print backtrace info [default: false]
  -c, --config-file <CONFIG_FILE>          Config file path [default: config.json]
      --log-level <LOG_LEVEL>              Log level [default: info]
      --base-log-level <BASE_LOG_LEVEL>    Base library log level [default: error]
  -h, --help                               Print help
  -V, --version                            Print version
```

---

## 🖥️ Client

### Running the Client

```bash
np_client run --server tcp://your-server:8118 --username user1 --password pass123
```

Multiple server addresses can be specified; the client will cycle through them on reconnection:

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
  -s, --server <SERVER>                    Server address (comma-separated, round-robin reconnect)
  -u, --username <USERNAME>                Username
  -p, --password <PASSWORD>                Password
      --enable-tls                         Enable TLS
      --tls-server-name <NAME>             TLS SNI server name (optional)
      --insecure                           Skip server certificate verification (not recommended)
      --ca-cert <CA_CERT>                  CA certificate file path
      --transport-max-connections <N>      Max forward connections/streams; 0 = single-connection mode [default: 16]
      --transport-min-connections <N>      Min connections to keep alive (pre-warmed); 0 = no warm-up [default: 4]
      --transport-idle-timeout-secs <SECS> Forward connection idle timeout in seconds [default: 60]
      --log-level <LOG_LEVEL>              Log level [default: info]
      --base-log-level <BASE_LOG_LEVEL>    Base library log level [default: error]
      --log-dir <LOG_DIR>                  Log directory [default: logs]
      --quiet                              Quiet mode, suppress log output
      --backtrace <BACKTRACE>              Print backtrace info [default: false]
  -h, --help                               Print help
```

### Windows Service (Windows only)

Run the following commands in an elevated command prompt to register the client as a Windows system service:

```bat
:: Install service
np_client.exe install --server tcp://your-server:8118 --username user1 --password pass123

:: Start service
sc.exe start "np_client"

:: Stop service
sc.exe stop "np_client"

:: Uninstall service
np_client.exe uninstall
```

---

## 🔐 TLS Certificate Generation

The project includes a certificate generation script for creating self-signed CA and server certificates:

```bash
./generate-certificate.sh
```

Generated files:

| File               | Purpose                    |
|--------------------|----------------------------|
| `root-ca.pem`      | Root CA certificate (client trust) |
| `root-ca.key.pem`  | Root CA private key        |
| `cert.pem`         | Server certificate         |
| `server.key.pem`   | Server private key         |

---

## 🌐 Web Dashboard

The web dashboard is built with **Vue 3 + Vite**, source code in `npipe-admin/`.

```bash
# Install dependencies
cd npipe-admin
npm install

# Development mode
npm run dev

# Production build (output to dist/)
npm run build
```

Configure the build output directory in the server's `web_base_dir`, then access `http://<server-ip>:<web_port>` to open the dashboard.

---

## 📄 License

This project is open-sourced under the [LICENSE](./LICENSE).

---

## 🔗 Links

- Project homepage: [https://github.com/tkzcfc/npipe](https://github.com/tkzcfc/npipe)
