mod callback_test;

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{sleep, Duration};

struct Context {
    name: String,
}

async fn test_rwlock(ctx: RwLock<Context>) {
    {
        for i in 0..10 {
            println!("for:{} - {}", i, ctx.read().await.name);
            ctx.write().await.name = format!("vvv-{}", i);
        }
    }
}

pub type OutputFuncType =
    Arc<dyn Fn(String) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send>> + Send + Sync>;
impl Context {
    async fn test_callback(&self, callback: OutputFuncType) -> anyhow::Result<()> {
        callback(self.name.clone()).await
    }
}

#[tokio::main]
async fn main() {
    test_rwlock(RwLock::new(Context {
        name: "haha".into(),
    }))
    .await;

    callback_test::run().await;
    let mtx = Mutex::new(0);

    tokio::join!(work(&mtx), work(&mtx));

    println!("{}", *mtx.lock().await);

    let name = String::from("aaa");
    let context_map = Arc::new(Mutex::new(HashMap::new()));
    context_map
        .lock()
        .await
        .insert(name.clone(), Context { name: name.clone() });

    let context_map_cloned = context_map.clone();
    let callback: OutputFuncType = Arc::new(move |key: String| {
        let context_map_cloned = context_map_cloned.clone();
        Box::pin(async move {
            // locked
            context_map_cloned.lock().await.remove(&key);
            Ok(())
        })
    });

    let ctx = context_map.lock().await.get(&name);
    let result = context_map
        .lock()
        .await
        .get(&name)
        .unwrap()
        .test_callback(callback)
        .await;
    if result.is_ok() {
        println!("ok!");
    }
}

async fn work(mtx: &Mutex<i32>) {
    println!("lock");
    {
        let mut v = mtx.lock().await;
        println!("locked");
        // slow redis network request
        sleep(Duration::from_millis(100)).await;
        *v += 1;
    }
    println!("unlock")
}
