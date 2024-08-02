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

------

## 隧道配置

| 名称              | 含义                                                      |
| ----------------- | --------------------------------------------------------- |
| source            | 隧道入口监听地址                                          |
| endpoint          | 隧道出口地址,SOCKS5类型此字段无效，随便写一个合法格式即可 |
| enabled           | 是否启用                                                  |
| compressed        | 是否压缩（使用lz4压缩）                                   |
| sender            | 隧道出口用户id(发送请求那一方)，为0则表示是出口在服务端   |
| receiver          | 隧道入口用户id（接收监听那一方）,为0则表示入口在服务端    |
| description       | 描述字段                                                  |
| tunnel_type       | 隧道类型 TCP  UDP  SOCKS5                                 |
| username          | SOCKS5代理认证用户名                                      |
| password          | SOCKS5代理认证密码                                        |
| encryption_method | 隧道加密方式                                              |
| custom_mapping    | 自定义域名                                                |

```
如：
   source配置 0.0.0.0:3000
   endpoint配置 www.baidu.com:80
   sender配置 1234（如1234是用户xxx的id）
   receiver配置 0
   
启动 np_client登录用户xxx
在np_client所在的电脑上访问 127.0.0.1:3000 即代表从服务端访问 www.baidu.com:80

```



