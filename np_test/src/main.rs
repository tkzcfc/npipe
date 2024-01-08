mod callback_test;

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
