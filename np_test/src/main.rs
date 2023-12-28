mod callback_test;

use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() {
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
