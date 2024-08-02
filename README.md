# npipe



## 客户端

```
Usage: np_client.exe [OPTIONS] --server <SERVER> --username <USERNAME> --password <PASSWORD>
```

------



## 服务端

### 服务端配置文件

| 名称         | 含义                                | 示例                                                         |
| ------------ | ----------------------------------- | ------------------------------------------------------------ |
| database_url | 数据库地址                          | sqlite格式 sqlite://data.db?mode=rwc<br />mysql格式 mysql://username:password@server:port/dbname, 如:mysql://admin:password@127.0.0.1:3306/npipe |
| listen_addr  | 服务端监听地址                      | 0.0.0.0:8118                                                 |
| web_base_dir | web后台管理路径 (为空则关闭web管理) | ./dist                                                       |
| web_addr     | web管理监听地址                     | 0.0.0.0:8120                                                 |
| web_username | web界面管理账号 (为空则关闭web管理) | admin                                                        |
| web_password | web界面管理密码 (为空则关闭web管理) | admin@1234                                                   |

### 使用方法

```
1. 启动服务器 ./np_server

2. 访问web管理后台 127.0.0.1:8120，添加用户和隧道
```







