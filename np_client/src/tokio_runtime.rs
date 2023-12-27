use std::{mem};


static mut TOKIO_RUNTIME: Option<&'static mut tokio::runtime::Runtime> = Option::None;

pub fn instance() -> &'static mut tokio::runtime::Runtime {
    unsafe {
        match TOKIO_RUNTIME {
            Option::Some(ref mut manager) => *manager,
            Option::None => {
                println!("new instance!");
                let manager_box = Box::new(tokio::runtime::Runtime::new().unwrap());
                let manager_raw = Box::into_raw(manager_box);
                TOKIO_RUNTIME = Some(&mut *manager_raw);
                &mut *manager_raw  // 如果不存在，先创建新的实例，然后返回
            }
        }
    }
}

pub fn destroy() {
    unsafe {
        if let Some(raw) = mem::replace(&mut TOKIO_RUNTIME, None) {
            drop(Box::from_raw(raw));
        }
    }
}