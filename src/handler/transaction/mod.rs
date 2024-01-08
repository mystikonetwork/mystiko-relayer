pub mod handler;

use crate::database::transaction::Transaction as DocumentTransaction;
use crate::error::RelayerServerError;
use async_trait::async_trait;
pub use handler::*;
use mystiko_relayer_types::TransactRequestData;
use mystiko_storage::Document;
use std::fmt::Debug;

#[async_trait]
pub trait TransactionHandler<T>: Debug + Send + Sync {
    type Error: Debug + Send;

    async fn create_by_request(&self, data: TransactRequestData) -> Result<T, Self::Error>;

    async fn find_by_id(&self, id: &str) -> Result<Option<T>, Self::Error>;

    async fn update_by_id(&self, id: &str, options: &UpdateTransactionOptions) -> Result<Option<T>, Self::Error>;

    async fn is_repeated_transaction(&self, signature: &str) -> Result<bool, Self::Error>;
}

#[async_trait]
impl TransactionHandler<Document<DocumentTransaction>>
    for Box<dyn TransactionHandler<Document<DocumentTransaction>, Error = RelayerServerError>>
{
    type Error = RelayerServerError;

    async fn create_by_request(&self, data: TransactRequestData) -> Result<Document<DocumentTransaction>, Self::Error> {
        self.as_ref().create_by_request(data).await
    }

    async fn find_by_id(&self, id: &str) -> Result<Option<Document<DocumentTransaction>>, Self::Error> {
        self.as_ref().find_by_id(id).await
    }

    async fn update_by_id(
        &self,
        id: &str,
        options: &UpdateTransactionOptions,
    ) -> Result<Option<Document<DocumentTransaction>>, Self::Error> {
        self.as_ref().update_by_id(id, options).await
    }

    async fn is_repeated_transaction(&self, signature: &str) -> Result<bool, Self::Error> {
        self.as_ref().is_repeated_transaction(signature).await
    }
}
