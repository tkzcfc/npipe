use std::cell::RefCell;
use std::{env, mem};
use std::fmt::{Debug, Display, format};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::{Arc, RwLock};
use log::info;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use std::sync::Mutex;
use np_base::{Result, server};
use np_base::connection::Connection;
use lazy_static::lazy_static;
use np_base::server::TcpServer;


trait Observer: Send + Sync {
    fn update(&self, msg: &str);
}

trait Observerable {
    fn register_observer(&mut self, obj: Box<dyn Observer + Send + Sync>);
    fn remove_observer(&mut self, index: usize);
    fn notify_observer(&self);
}

struct WechatServer {
    msg: &'static str,
    list: Vec<Box<dyn Observer + Send + Sync>>
}

impl WechatServer {
    fn new() -> WechatServer{
        return WechatServer{
            list: Vec::new(),
            msg: ""
        };
    }

    // 发送消息通知
    fn set_data_msg(&mut self, msg: &'static str) {
        self.msg = msg;
        self.notify_observer();
    }
}

impl Observerable for WechatServer {
    fn register_observer(&mut self, obj: Box<dyn Observer + Send + Sync>) {
        self.list.push(obj)
    }

    fn remove_observer(&mut self, index: usize) {
        self.list.remove(index);
    }

    fn notify_observer(&self) {
        for obj in self.list.iter() {
            obj.update(self.msg);
        }
    }
}

struct User {
    name: &'static str
}

impl Observer for User{
    fn update(&self, msg: &str) {
        println!("{} -> 收到消息: {}", self.name, msg);
    }
}

#[tokio::main]
pub async fn main() -> Result<()> {
    env::set_var("RUST_LOG", "info");
    env_logger::init();

    let count = Arc::new(tokio::sync::RwLock::new(0));

    let count_copyed = Arc::clone(&count);

    let listener = TcpListener::bind("0.0.0.0:2000").await?;
    let server = TcpServer {};
    server.run(listener, |connection| async move {
        let mut connection = connection.write().await;

        // let mut n =  count_copyed.write().await;
        // *n = *n + 1;

        let content = String::from("ok");
        let response = format!("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}", content.len(), content);
        connection.stream.write(response.as_bytes()).await?;
        connection.stream.flush().await?;
        connection.stream.shutdown().await?;
        info!("send ok!");
        Ok(())
    }).await?;

    // let count = Arc::new(tokio::sync::RwLock::new(0));
    // tokio::spawn(async move {
    //     let mut n =  count.write().await;
    //     *n = *n + 1;
    // }).await;

    let server = Arc::new(Mutex::new(WechatServer::new()));
    server.lock().unwrap().register_observer(Box::new(User{name:"张三"}));
    server.lock().unwrap().register_observer(Box::new(User{name:"李四"}));
    server.lock().unwrap().register_observer(Box::new(User{name:"王五"}));

    let mut server_clone = Arc::clone(&server);
    tokio::spawn(async move {
        server_clone.lock().unwrap().set_data_msg("这是测试消息");
        // server.register_observer(Box::new(User{name:"赵六"}));
        server_clone.lock().unwrap().remove_observer(1);
        // server.register_observer(Box::new(User{name:"田七"}));
        // server.set_data_msg("rust是最棒的");
    });
    // println!("----------------->>>");
    Ok(())
}
