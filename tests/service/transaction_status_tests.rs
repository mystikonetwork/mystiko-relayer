use crate::common::TestServer;
use actix_web::test::{call_and_read_body_json, init_service, TestRequest};
use actix_web::web::Data;
use actix_web::App;
use mystiko_relayer::database::Database;
use mystiko_relayer::error::ResponseError;
use mystiko_relayer::handler::transaction::TransactionHandler;
use mystiko_relayer::service::transaction_status;
use mystiko_relayer_types::response::{ApiResponse, ResponseCode};
use mystiko_relayer_types::{RelayTransactStatusResponse, TransactRequestData, TransactStatus};
use mystiko_storage::SqlStatementFormatter;
use mystiko_storage_sqlite::SqliteStorageBuilder;
use mystiko_types::{BridgeType, CircuitType, TransactionType};
use serial_test::file_serial;
use std::sync::Arc;

#[actix_rt::test]
async fn test_successful() {
    // create test server
    let server = TestServer::singleton().await;
    // init service
    let app = init_service(
        App::new()
            .app_data(Data::new(server.transaction_handler.clone()))
            .service(transaction_status),
    )
    .await;

    // insert raw transaction data
    let request = TransactRequestData {
        contract_param: Default::default(),
        transaction_type: TransactionType::Withdraw,
        bridge_type: BridgeType::Loop,
        chain_id: 5,
        asset_symbol: "MTT".to_string(),
        asset_decimals: 16,
        pool_address: "0x4F416Acfd1153F9Af782056e68607227Af29D931".to_string(),
        circuit_type: CircuitType::Transaction1x0,
        signature: "0x800157ae47e94a156c42584190c33362b13ff94a7e8f5ef6ffd602c8d19ae\
        0684a4da6afd3c10bae9bd252dd20a9388d86c617bacb807a236a0385603e4086d61c"
            .to_string(),
    };
    let insert = server.transaction_handler.create_by_request(request).await;
    assert!(insert.is_ok());
    let id = insert.unwrap().id;

    let req = TestRequest::get()
        .uri(&format!("/transaction/status/{}", id))
        .to_request();
    let resp: ApiResponse<RelayTransactStatusResponse> = call_and_read_body_json(&app, req).await;
    assert_eq!(resp.code, ResponseCode::Successful as i32);
    assert!(resp.message.is_none());
    assert!(resp.data.is_some());

    let info = resp.data.unwrap();
    assert_eq!(info.uuid, id);
    assert_eq!(info.status, TransactStatus::Queued);
}

#[actix_rt::test]
async fn test_id_not_found() {
    // create test server
    let server = TestServer::singleton().await;
    // init service
    let app = init_service(
        App::new()
            .app_data(Data::new(server.transaction_handler.clone()))
            .service(transaction_status),
    )
    .await;

    let req = TestRequest::get().uri("/transaction/status/123456").to_request();
    let resp: ApiResponse<RelayTransactStatusResponse> = call_and_read_body_json(&app, req).await;
    assert_eq!(resp.code, ResponseCode::TransactionNotFound as i32);
    assert!(resp.message.is_some());
    assert_eq!(
        resp.message.unwrap(),
        ResponseError::TransactionNotFound {
            id: "123456".parse().unwrap()
        }
        .to_string()
    );
}

#[actix_rt::test]
#[file_serial]
async fn test_db_error() {
    // create test server
    let mut server = TestServer::new(None).await.unwrap();
    let database = Database::new(
        SqlStatementFormatter::sqlite(),
        SqliteStorageBuilder::new().in_memory().build().await.unwrap(),
    );
    let transaction_handler = TransactionHandler::new(Arc::new(database));
    server.transaction_handler = Arc::new(transaction_handler);
    // init service
    let app = init_service(
        App::new()
            .app_data(Data::new(server.transaction_handler.clone()))
            .service(transaction_status),
    )
    .await;
    let id = "test_id1234";

    let req = TestRequest::get()
        .uri(&format!("/transaction/status/{}", id))
        .to_request();
    let resp: ApiResponse<RelayTransactStatusResponse> = call_and_read_body_json(&app, req).await;
    assert_eq!(resp.code, ResponseCode::DatabaseError as i32);
}
