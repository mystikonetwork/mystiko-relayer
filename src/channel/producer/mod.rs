use crate::database::transaction::Transaction;
use async_trait::async_trait;
use mystiko_relayer_types::TransactRequestData;
use mystiko_storage::Document;

pub mod handler;

#[async_trait]
pub trait ProducerHandler: Send + Sync {
    type Error;

    async fn send(&self, data: TransactRequestData) -> Result<Document<Transaction>, Self::Error>;
}
