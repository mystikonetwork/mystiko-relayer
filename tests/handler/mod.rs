use async_trait::async_trait;
use mockall::mock;
use mystiko_relayer::database::transaction::Transaction;
use mystiko_relayer::error::RelayerServerError;
use mystiko_relayer::handler::transaction::{TransactionHandler, UpdateTransactionOptions};
use mystiko_relayer_types::TransactRequestData;
use mystiko_storage::Document;

mod account;
mod transaction;

mock! {
    #[derive(Debug)]
    pub Transactions {}

    #[async_trait]
    impl TransactionHandler<Document<Transaction>> for Transactions {
        type Error = RelayerServerError;
        async fn create_by_request(&self, data: TransactRequestData) -> Result<Document<Transaction>, RelayerServerError>;
        async fn find_by_id(&self, id: &str) -> Result<Option<Document<Transaction>>, RelayerServerError>;
        async fn update_by_id(
            &self,
            id: &str,
            options: &UpdateTransactionOptions,
        ) -> Result<Option<Document<Transaction>>, RelayerServerError>;
        async fn is_repeated_transaction(&self, signature: &str) -> Result<bool, RelayerServerError>;
    }
}

mock! {
    #[derive(Debug)]
    pub Accounts {}

    #[async_trait]
    impl AccountHandler<Document<Account>> for Accounts {}
}
