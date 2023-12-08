use std::cell::RefCell;
use std::{env, mem};
use std::fmt::{Debug, Display};
use std::sync::Arc;
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

fn test_func_1(param1: &(impl Clone + Debug + Display), param2: & mut(impl Clone + Display)) -> i32 {
    0
}

fn test_func_2<T: Clone + Debug + Display, U: Clone + Display>(param1: &T, param2: &mut U) -> i32 {
    0
}

fn test_func_3<T, U>(param1: &T, param2: &mut U) -> i32
    where T: Clone + Debug + Display,
          U: Clone + Display
{
    0
}

#[tokio::main]
pub async fn main() -> Result<()> {
    env::set_var("RUST_LOG", "info");
    env_logger::init();

    let mut var = String::from("aaa");
    test_func_1(&String::from("aaaa"), &mut var);
    test_func_2(&String::from("aaaa"), &mut var);
    test_func_3(&String::from("aaaa"), &mut var);

    // let listener = TcpListener::bind("0.0.0.0:2000").await?;
    // let server = TcpServer {};
    // server.run(listener, |connection| async {
    //             // connection.stream.write_all(b"ok!").await;
    //             // connection.stream.flush().await;
    //             // connection.stream.shutdown().await;
    //             // info!("write finish: {}", connection.addr);
    // }).await?;

    // tokio::spawn(async {
    //     MANAGER_INSTANCE.lock().unwrap().call_me();
    // }).await;

    // let listener = TcpListener::bind("0.0.0.0:2000").await?;
    //
    // server::run(listener, |mut connection: Connection| {
    //     info!("new connection: {}", connection.addr);
    //     tokio::spawn(async move {
    //         connection.stream.write_all(b"ok!").await;
    //         connection.stream.flush().await;
    //         connection.stream.shutdown().await;
    //         info!("write finish: {}", connection.addr);
    //     });
    // }).await?;

    // let server = Arc::new(Mutex::new(WechatServer::new()));
    // server.lock().unwrap().register_observer(Box::new(User{name:"张三"}));
    // server.lock().unwrap().register_observer(Box::new(User{name:"李四"}));
    // server.lock().unwrap().register_observer(Box::new(User{name:"王五"}));
    //
    // let mut server_clone = Arc::clone(&server);
    // tokio::spawn(async move {
    //     server_clone.lock().unwrap().set_data_msg("这是测试消息");
    //     // server.register_observer(Box::new(User{name:"赵六"}));
    //     // server.remove_observer(1);
    //     // server.register_observer(Box::new(User{name:"田七"}));
    //     // server.set_data_msg("rust是最棒的");
    // });
    // println!("----------------->>>");
    Ok(())
}
