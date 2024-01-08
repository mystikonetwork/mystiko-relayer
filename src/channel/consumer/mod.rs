use async_trait::async_trait;

pub mod handler;

#[async_trait]
pub trait ConsumerHandler: Send + Sync {
    async fn consume(&mut self);
}
