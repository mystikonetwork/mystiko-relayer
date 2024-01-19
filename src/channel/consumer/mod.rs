use async_trait::async_trait;

pub mod handler;

#[async_trait]
pub trait ConsumerHandler: Send + Sync {
    async fn consume(&mut self);
}

#[async_trait]
impl ConsumerHandler for Box<dyn ConsumerHandler> {
    async fn consume(&mut self) {
        (**self).consume().await;
    }
}
