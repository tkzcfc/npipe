use std::future::Future;

pub trait Transport: Send + Sync {
    type Fut: Future<Output = crate::Result<()>>;

    // 写逻辑
    fn read(&mut self) -> Self::Fut;
}
