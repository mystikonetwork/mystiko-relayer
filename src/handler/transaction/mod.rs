pub mod handler;

use async_trait::async_trait;
pub use handler::*;
use mystiko_relayer_types::TransactRequestData;
use std::fmt::Debug;

#[async_trait]
pub trait TransactionHandler<T>: Debug + Send + Sync {
    type Error: Debug + Send;

    async fn create_by_request(&self, data: TransactRequestData) -> Result<T, Self::Error>;

    async fn find_by_id(&self, id: &str) -> Result<Option<T>, Self::Error>;

    async fn update_by_id(&self, id: &str, options: &UpdateTransactionOptions) -> Result<Option<T>, Self::Error>;

    async fn is_repeated_transaction(&self, signature: &str) -> Result<bool, Self::Error>;
}
