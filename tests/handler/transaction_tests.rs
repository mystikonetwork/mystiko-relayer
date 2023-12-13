use crate::common::TestServer;
use mystiko_protos::core::v1::SpendType;
use mystiko_relayer::handler::transaction::UpdateTransactionOptions;
use mystiko_relayer_types::{TransactRequestData, TransactStatus};
use mystiko_types::CircuitType;

#[actix_rt::test]
async fn update_by_id() {
    let server = TestServer::new(None).await.unwrap();
    let transaction_handler = server.transaction_handler.clone();
    let doc = transaction_handler
        .create_by_request(TransactRequestData {
            contract_param: Default::default(),
            spend_type: SpendType::Transfer,
            bridge_type: Default::default(),
            chain_id: 0,
            asset_symbol: "".to_string(),
            asset_decimals: 0,
            pool_address: "".to_string(),
            circuit_type: CircuitType::Rollup1,
            signature: "".to_string(),
        })
        .await
        .unwrap();
    let id = doc.id;

    let options = UpdateTransactionOptions {
        status: Some(TransactStatus::Failed),
        error_message: Some("error_message".to_string()),
        transaction_hash: Some("transaction_hash".to_string()),
    };
    let result = transaction_handler.update_by_id(&id, &options).await.unwrap().unwrap();
    assert_eq!(result.data.status, TransactStatus::Failed);
    assert_eq!(result.data.error_message.unwrap(), "error_message");
    assert_eq!(result.data.transaction_hash.unwrap(), "transaction_hash");
}

#[actix_rt::test]
async fn no_change_update() {
    let server = TestServer::new(None).await.unwrap();
    let transaction_handler = server.transaction_handler.clone();
    let doc = transaction_handler
        .create_by_request(TransactRequestData {
            contract_param: Default::default(),
            spend_type: SpendType::Transfer,
            bridge_type: Default::default(),
            chain_id: 0,
            asset_symbol: "".to_string(),
            asset_decimals: 0,
            pool_address: "".to_string(),
            circuit_type: CircuitType::Rollup1,
            signature: "".to_string(),
        })
        .await
        .unwrap();
    let id = doc.id;

    let result = transaction_handler
        .update_by_id(
            &id,
            &UpdateTransactionOptions {
                status: Some(doc.data.status),
                error_message: doc.data.error_message,
                transaction_hash: doc.data.transaction_hash,
            },
        )
        .await;
    assert!(result.is_ok());
}

#[actix_rt::test]
async fn update_not_exist_id() {
    let server = TestServer::new(None).await.unwrap();
    let transaction_handler = server.transaction_handler.clone();
    let result = transaction_handler
        .update_by_id(
            "123",
            &UpdateTransactionOptions {
                status: None,
                error_message: None,
                transaction_hash: None,
            },
        )
        .await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[actix_rt::test]
async fn update_exist_error_msg() {
    let server = TestServer::new(None).await.unwrap();
    let transaction_handler = server.transaction_handler.clone();
    let doc = transaction_handler
        .create_by_request(TransactRequestData {
            contract_param: Default::default(),
            spend_type: SpendType::Transfer,
            bridge_type: Default::default(),
            chain_id: 0,
            asset_symbol: "".to_string(),
            asset_decimals: 0,
            pool_address: "".to_string(),
            circuit_type: CircuitType::Rollup1,
            signature: "".to_string(),
        })
        .await
        .unwrap();
    let id = doc.id;

    let options = UpdateTransactionOptions {
        status: Some(TransactStatus::Failed),
        error_message: Some("error_message".to_string()),
        transaction_hash: None,
    };
    let result = transaction_handler.update_by_id(&id, &options).await.unwrap().unwrap();
    assert_eq!(result.data.status, TransactStatus::Failed);
    assert_eq!(result.data.error_message.unwrap(), "error_message");

    let options = UpdateTransactionOptions {
        status: Some(TransactStatus::Failed),
        error_message: Some("error_message".to_string()),
        transaction_hash: None,
    };
    let result = transaction_handler.update_by_id(&id, &options).await.unwrap().unwrap();
    assert_eq!(result.data.status, TransactStatus::Failed);
    assert_eq!(result.data.error_message.unwrap(), "error_message");
}
