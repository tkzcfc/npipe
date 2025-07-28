# npipe

npipe is  is cross platform (Windows, Linux, OSX) 

It provides simple and efficient ways to forward data from multiple sockets (TCP or UDP) through a single secure TLS tunnel to a remote computer.

Features:
* Local and remote TCP port forwarding
* Local and remote UDP port forwarding
* Local and remote SOCKS server
* Local and remote HTTP Proxy server
* TLS connection with the strongest cipher-suites

## How to use

### Command line

#### Client

```
Usage: np_client.exe run [OPTIONS] --server <SERVER> --username <USERNAME> --password <PASSWORD>

Options:
      --backtrace <BACKTRACE>
          print backtracking information [default: false] [possible values: true, false]
  -s, --server <SERVER>
          server address
  -u, --username <USERNAME>
          username
  -p, --password <PASSWORD>
          password
      --enable-tls
          enable tls
      --insecure
          if true, the validity of the SSL certificate is not verified
      --quiet
          Quiet mode. Do not print logs
      --ca-cert <CA_CERT>
          ca file path (optional), if not provided, the client’s certificate will not be verified [default: ]
      --log-level <LOG_LEVEL>
          set log level [default: info]
      --base-log-level <BASE_LOG_LEVEL>
          set log level [default: error]
      --log-dir <LOG_DIR>
          set log directory [default: logs]
  -h, --help
          Print help (see more with '--help')
```

Register the client as a service on Windows (must be executed in a console with administrator privileges)

```
Usage: np_client.exe install [OPTIONS] --server <SERVER> --username <USERNAME> --password <PASSWORD>

Options:
      --backtrace <BACKTRACE>
          print backtracking information [default: false] [possible values: true, false]
  -s, --server <SERVER>
          server address
  -u, --username <USERNAME>
          username
  -p, --password <PASSWORD>
          password
      --enable-tls
          enable tls
      --insecure
          if true, the validity of the SSL certificate is not verified
      --quiet
          Quiet mode. Do not print logs
      --ca-cert <CA_CERT>
          ca file path (optional), if not provided, the client’s certificate will not be verified [default: ]
      --log-level <LOG_LEVEL>
          set log level [default: info]
      --base-log-level <BASE_LOG_LEVEL>
          set log level [default: error]
      --log-dir <LOG_DIR>
          set log directory [default: logs]
  -h, --help
          Print help (see more with '--help')
```

Then start the service and typing: 

```
sc.exe start "np_client"
```

Example usage：

```
```



#### Server

```
Usage: np_server.exe [OPTIONS]

Options:
  -b, --backtrace <BACKTRACE>
          Print backtracking information [default: false] [possible values: true, false]
  -c, --config-file <CONFIG_FILE>
          Config file [default: config.json]
      --log-level <LOG_LEVEL>
          Set log level  warn [default: info]
      --base-log-level <BASE_LOG_LEVEL>
          Set log level [default: error]
  -h, --help
          Print help
  -V, --version
          Print version
```



### Configuration file

```json
{
	"database_url": "sqlite://data.db?mode=rwc",
	"listen_addr": "tcp://0.0.0.0:8118,kcp://0.0.0.0:8118,,ws://0.0.0.0:8119",
	"illegal_traffic_forward": "",
	"enable_tls": false,
	"tls_cert": "./cert.pem",
	"tls_key": "./server.key.pem",
	"web_base_dir": "./dist",
	"web_addr": "0.0.0.0:8120",
	"web_username": "admin",
	"web_password": "admin@1234"
}
```

#### Arguments

### 

| Configuration key       | Description                                                  | Example                                                      |
| ----------------------- | ------------------------------------------------------------ | ------------------------------------------------------------ |
| database_url            | database address                                             | sqlite://data.db?mode=rwc<br> mysql://username:password@server:port/dbname, |
| listen_addr             | Server listening address(Multiple addresses should be separated by commas) | tcp://0.0.0.0:8118,kcp://0.0.0.0:8118                        |
| enable_tls              | Enable TLS connection                                        | true/false                                                   |
| tls_cert                | Cert file path                                               | ./cert.pem                                                   |
| tls_key                 | Key file path                                                | ./server.key.pem                                             |
| web_base_dir            | Web backend management path (if empty, close web management) | ./dist                                                       |
| web_addr                | Web management listening address                             | 0.0.0.0:8120                                                 |
| web_username            | Web interface management account (if left blank, close web management) | admin                                                        |
| web_password            | Web interface management password (if left blank, turn off web management) | admin@1234                                                   |
| illegal_traffic_forward | Illegal traffic request forwarding address                   | You can forward traffic that is not npipe to other programs, such as nginx. Configuration format example: 127.0.0.1:80. If it is empty, do not forward the request |
| quiet                   | Quiet mode. Do not print logs                                | true/false                                                   |
| log_dir                 | Log saving directory                                         | logs                                                         |



## How to generate certificates for TLS connections

```bash
./generate-certificate.sh
```

------

[benchmark](./benchmark.md)

------

Thanks to [pizixi](https://github.com/pizixi) for developing the [ admin interface](https://github.com/pizixi/npipe-webui)
