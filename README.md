# npipe







### .env 文件内容

安装sqlx-cli
cargo install sqlx-cli --features mysql

更新离线模式缓存
cargo sqlx prepare --database-url mysql://npipe:np%40123@192.168.175.129:5306/npipe

```

DATABASE_URL=mysql://npipe:np%40123@192.168.175.129:5306/npipe


离线模式
SQLX_OFFLINE=true

```

