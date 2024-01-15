use crate::channel::MockConsumers;
use crate::common::{default_transaction, MockTokenPrice};
use crate::handler::{MockAccounts, MockTransactions};
use crate::service::{create_app, MockOptions};
use actix_web::test::{call_and_read_body_json, TestRequest};
use mystiko_relayer::error::RelayerServerError;
use mystiko_relayer::service::v1::response::JobStatusResponse;
use mystiko_relayer_types::response::{ApiResponse, ResponseCode};
use mystiko_storage::{Document, StorageError};
use std::collections::HashMap;

#[actix_rt::test]
async fn test_success() {
    let mut transaction_handler = MockTransactions::new();
    transaction_handler
        .expect_find_by_id()
        .withf(|id| id == "1")
        .returning(|id| {
            let transaction = default_transaction();
            Ok(Some(Document::new(
                id.to_string(),
                1234567890u64,
                1234567891u64,
                transaction,
            )))
        });
    let options = MockOptions {
        providers: HashMap::new(),
        transaction_handler,
        account_handler: MockAccounts::new(),
        token_price: MockTokenPrice::new(),
        consumer: MockConsumers::new(),
    };
    let app = create_app(options).await.unwrap();

    let request = TestRequest::get().uri("/jobs/1").to_request();
    let response: ApiResponse<JobStatusResponse> = call_and_read_body_json(&app, request).await;
    assert_eq!(response.code, ResponseCode::Successful as i32);
    assert!(response.data.is_some());
    assert_eq!(response.data.unwrap().id, "1");
}

#[actix_rt::test]
async fn test_with_id_not_found() {
    let mut transaction_handler = MockTransactions::new();
    transaction_handler
        .expect_find_by_id()
        .withf(|id| id != "1")
        .returning(|_| Ok(None));
    let options = MockOptions {
        providers: HashMap::new(),
        transaction_handler,
        account_handler: MockAccounts::new(),
        token_price: MockTokenPrice::new(),
        consumer: MockConsumers::new(),
    };
    let app = create_app(options).await.unwrap();
    let request = TestRequest::get().uri("/jobs/2").to_request();
    let response: ApiResponse<JobStatusResponse> = call_and_read_body_json(&app, request).await;
    assert_eq!(response.code, ResponseCode::TransactionNotFound as i32);
    assert!(response.data.is_none());
}

#[actix_rt::test]
async fn test_with_error() {
    let mut transaction_handler = MockTransactions::new();
    transaction_handler.expect_find_by_id().returning(|_| {
        Err(RelayerServerError::StorageError(StorageError::NoSuchColumnError(
            "mock_error".to_string(),
        )))
    });
    let options = MockOptions {
        providers: HashMap::new(),
        transaction_handler,
        account_handler: MockAccounts::new(),
        token_price: MockTokenPrice::new(),
        consumer: MockConsumers::new(),
    };
    let app = create_app(options).await.unwrap();
    let request = TestRequest::get().uri("/jobs/2").to_request();
    let response: ApiResponse<JobStatusResponse> = call_and_read_body_json(&app, request).await;
    assert_eq!(response.code, ResponseCode::DatabaseError as i32);
    assert_eq!(response.message.unwrap(), "relayer server database error");
}
