use std::future::Future;
use std::pin::Pin;
use std::boxed::Box;

// 定义一个 trait 来代表接收 u32 并返回一个异步执行的函数。
trait AsyncFnOneResult {
    fn call(&self, arg: u32) -> Pin<Box<dyn Future<Output = ()> + Send>>;
}

// 实现 AsyncFnOneResult 为满足特定签名的闭包。
impl<F, Fut> AsyncFnOneResult for F
    where
        F: Fn(u32) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
{
    fn call(&self, arg: u32) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        Box::pin(self(arg))
    }
}

// do_something 函数接受一个实现了 AsyncFnOneResult trait 的类型。
async fn do_something(callback: impl AsyncFnOneResult + 'static) {
    callback.call(0u32).await;
}

// 你的 async_callback 函数。
async fn async_callback(num: u32) {
    println!("call me !!!   {}", num);
}

pub async fn run() {
    do_something(async_callback).await;

    do_something(|num| async move {
        println!("call --------------->>  num: {}", num);
    }).await;
}
