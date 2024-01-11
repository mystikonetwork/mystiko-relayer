use crate::common::{create_default_database_in_memory, default_transact_request_data};
use mystiko_protos::core::v1::SpendType;
use mystiko_relayer::handler::transaction::{Transaction, TransactionHandler, UpdateTransactionOptions};
use mystiko_relayer_types::TransactStatus;
use mystiko_types::{BridgeType, CircuitType};
use std::sync::Arc;

#[actix_rt::test]
async fn test_create_by_request() {
    let chain_id = 99;
    let db = create_default_database_in_memory().await;
    let handler = Transaction::new(Arc::new(db));
    let result = handler.create_by_request(default_transact_request_data(chain_id)).await;
    assert!(result.is_ok());
    let transaction = result.unwrap();
    assert_eq!(transaction.data.chain_id, chain_id);
    assert_eq!(transaction.data.spend_type, SpendType::Withdraw);
    assert_eq!(transaction.data.bridge_type, BridgeType::Loop);
    assert_eq!(transaction.data.status, TransactStatus::Queued);
    assert_eq!(
        transaction.data.pool_address,
        "0x4F416Acfd1153F9Af782056e68607227Af29D931"
    );
    assert_eq!(transaction.data.asset_symbol, "ETH");
    assert_eq!(transaction.data.asset_decimals, 18);
    assert_eq!(transaction.data.circuit_type, CircuitType::Transaction1x0);
    assert_eq!(transaction.data.transaction_hash, None);
}

#[actix_rt::test]
async fn test_find_by_id() {
    let chain_id = 99;
    let db = create_default_database_in_memory().await;
    let handler = Transaction::new(Arc::new(db));
    let result = handler.create_by_request(default_transact_request_data(chain_id)).await;
    assert!(result.is_ok());
    let transaction_0 = result.unwrap();
    let result = handler.find_by_id(transaction_0.id.as_str()).await;
    assert!(result.is_ok());
    let transaction_1 = result.unwrap().unwrap();
    assert_eq!(transaction_0, transaction_1);
}

#[actix_rt::test]
async fn test_update_by_id() {
    let chain_id = 99;
    let db = create_default_database_in_memory().await;
    let handler = Transaction::new(Arc::new(db));
    let result = handler.create_by_request(default_transact_request_data(chain_id)).await;
    assert!(result.is_ok());
    let transaction_0 = result.unwrap();
    let result = handler
        .update_by_id(
            transaction_0.id.as_str(),
            &UpdateTransactionOptions {
                status: Some(TransactStatus::Failed),
                error_message: Some("error_message".to_string()),
                transaction_hash: Some("transaction_hash".to_string()),
            },
        )
        .await;
    assert!(result.is_ok());
    let transaction_1 = result.unwrap().unwrap();
    assert_eq!(transaction_1.data.status, TransactStatus::Failed);
    assert_eq!(transaction_1.data.error_message.unwrap(), "error_message");
    assert_eq!(transaction_1.data.transaction_hash.unwrap(), "transaction_hash");
}

#[actix_rt::test]
async fn test_is_repeated_transaction() {
    let chain_id = 99;
    let db = create_default_database_in_memory().await;
    let handler = Transaction::new(Arc::new(db));
    let result = handler.create_by_request(default_transact_request_data(chain_id)).await;
    assert!(result.is_ok());
    let transaction_0 = result.unwrap();
    let result = handler
        .is_repeated_transaction(transaction_0.data.signature.as_str())
        .await;
    assert!(result.is_ok());
    assert!(result.unwrap());
    let result = handler.is_repeated_transaction("signature").await;
    assert!(result.is_ok());
    assert!(!result.unwrap());
}
