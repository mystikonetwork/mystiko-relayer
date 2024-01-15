use crate::database::transaction::Transaction;
use crate::error::RelayerServerError;
use async_trait::async_trait;
use mystiko_relayer_types::TransactRequestData;
use mystiko_storage::Document;

pub mod handler;

#[async_trait]
pub trait ProducerHandler: Send + Sync {
    type Error;

    async fn send(&self, data: TransactRequestData) -> Result<Document<Transaction>, Self::Error>;
}

#[async_trait]
impl ProducerHandler for Box<dyn ProducerHandler<Error = RelayerServerError>> {
    type Error = RelayerServerError;

    async fn send(&self, data: TransactRequestData) -> Result<Document<Transaction>, Self::Error> {
        self.as_ref().send(data).await
    }
}
