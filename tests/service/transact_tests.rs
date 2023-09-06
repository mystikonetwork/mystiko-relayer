use crate::common::{get_valid_transact_request_data, TestServer};
use actix_web::test::{call_and_read_body_json, init_service, TestRequest};
use actix_web::web::Data;
use actix_web::App;
use ethereum_types::U256;
use ethers_providers::MockProvider;
use mystiko_relayer::channel::transact_channel;
use mystiko_relayer::error::ResponseError;
use mystiko_relayer::service::{transact, transaction_status};
use mystiko_relayer_types::response::{ApiResponse, ResponseCode};
use mystiko_relayer_types::{RelayTransactResponse, RelayTransactStatusResponse, TransactRequestData, TransactStatus};
use mystiko_types::{BridgeType, CircuitType, TransactionType};
use serial_test::file_serial;

#[actix_rt::test]
#[file_serial]
async fn test_send_successful_main() {
    // create test server
    let mock_provider = MockProvider::new();
    let server = TestServer::new(Some(mock_provider.clone())).await.unwrap();

    // init service
    let app = init_service(
        App::new()
            .app_data(Data::new(server.app_state.clone()))
            .app_data(Data::new(server.senders.clone()))
            .app_data(Data::new(server.transaction_handler.clone()))
            .service(transact)
            .service(transaction_status),
    )
    .await;

    let gas = U256::from(100_000_000_000u64);
    let nonce = U256::from(100);
    let price = U256::from(1000000);
    mock_provider.push(price).unwrap();
    mock_provider.push(gas).unwrap();
    mock_provider.push(nonce).unwrap();
    mock_provider.push(price).unwrap();

    let req = TestRequest::post()
        .uri("/transact")
        .set_json(get_valid_transact_request_data())
        .to_request();
    let resp: ApiResponse<RelayTransactResponse> = call_and_read_body_json(&app, req).await;
    assert!(resp.message.is_none());
    assert!(resp.data.is_some());
    assert_eq!(resp.code, ResponseCode::Successful as i32);

    let uuid = resp.data.unwrap().uuid;
    let req = TestRequest::get()
        .uri(&format!("/transaction/status/{}", uuid))
        .to_request();
    let resp: ApiResponse<RelayTransactStatusResponse> = call_and_read_body_json(&app, req).await;
    assert!(resp.message.is_none());
    assert!(resp.data.is_some());
    assert_eq!(resp.code, ResponseCode::Successful as i32);
    let status = resp.data.unwrap().status;
    assert_eq!(&status, &TransactStatus::Queued);
}

#[actix_rt::test]
#[file_serial]
async fn test_send_successful_erc20() {
    // create test server
    let mock_provider = MockProvider::new();
    let server = TestServer::new(Some(mock_provider.clone())).await.unwrap();

    // init service
    let app = init_service(
        App::new()
            .app_data(Data::new(server.app_state.clone()))
            .app_data(Data::new(server.senders.clone()))
            .app_data(Data::new(server.transaction_handler.clone()))
            .service(transact)
            .service(transaction_status),
    )
    .await;

    let gas = U256::from(100_000_000_000u64);
    let nonce = U256::from(100);
    let price = U256::from(1000000);
    mock_provider.push(price).unwrap();
    mock_provider.push(gas).unwrap();
    mock_provider.push(nonce).unwrap();
    mock_provider.push(price).unwrap();

    let mut transact_data = get_valid_transact_request_data();
    transact_data.asset_symbol = "mtt".to_string();

    let req = TestRequest::post()
        .uri("/transact")
        .set_json(transact_data)
        .to_request();
    let resp: ApiResponse<RelayTransactResponse> = call_and_read_body_json(&app, req).await;
    assert!(resp.message.is_none());
    assert!(resp.data.is_some());
    assert_eq!(resp.code, ResponseCode::Successful as i32);

    let uuid = resp.data.unwrap().uuid;
    let req = TestRequest::get()
        .uri(&format!("/transaction/status/{}", uuid))
        .to_request();
    let resp: ApiResponse<RelayTransactStatusResponse> = call_and_read_body_json(&app, req).await;
    assert!(resp.message.is_none());
    assert!(resp.data.is_some());
    assert_eq!(resp.code, ResponseCode::Successful as i32);
    let status = resp.data.unwrap().status;
    assert_eq!(&status, &TransactStatus::Queued);
}

#[actix_rt::test]
async fn test_invalid_request() {
    // create test server
    let server = TestServer::singleton().await;
    // init service
    let app = init_service(
        App::new()
            .app_data(Data::new(server.app_state.clone()))
            .app_data(Data::new(server.senders.clone()))
            .app_data(Data::new(server.transaction_handler.clone()))
            .service(transact),
    )
    .await;

    let req = TestRequest::post()
        .uri("/transact")
        .set_json(TransactRequestData {
            contract_param: Default::default(),
            transaction_type: TransactionType::Withdraw,
            bridge_type: BridgeType::Loop,
            chain_id: 5,
            asset_symbol: "".to_string(),
            asset_decimals: 0,
            pool_address: "0x4F416Acfd1153F9Af782056e68607227Af29D931".to_string(),
            circuit_type: CircuitType::Rollup1,
            signature: "".to_string(),
        })
        .to_request();
    let resp: ApiResponse<RelayTransactResponse> = call_and_read_body_json(&app, req).await;
    assert!(resp.message.is_some());
    assert_eq!(resp.code, ResponseCode::ValidateError as i32);
    assert_eq!(
        resp.message.unwrap(),
        ResponseError::ValidateError {
            error: "circuit_type: Validation error: invalid circuit type [{\"value\": String(\"rollup1\")}]"
                .parse()
                .unwrap()
        }
        .to_string()
    );
}

#[actix_rt::test]
async fn test_repeated_transaction() {
    // create test server
    let server = TestServer::singleton().await;
    // init service
    let app = init_service(
        App::new()
            .app_data(Data::new(server.app_state.clone()))
            .app_data(Data::new(server.senders.clone()))
            .app_data(Data::new(server.transaction_handler.clone()))
            .service(transact),
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
    let result = server.transaction_handler.create_by_request(request.clone()).await;
    assert!(result.is_ok());

    // send repeated transaction
    let req = TestRequest::post().uri("/transact").set_json(request).to_request();
    let resp: ApiResponse<RelayTransactResponse> = call_and_read_body_json(&app, req).await;
    assert_eq!(resp.code, ResponseCode::RepeatedTransaction as i32);
    assert!(resp.message.is_some());
    assert_eq!(resp.message.unwrap(), ResponseError::RepeatedTransaction.to_string());
}

#[actix_rt::test]
async fn test_chain_id_not_found() {
    // create test server
    let server = TestServer::singleton().await;
    // init service
    let app = init_service(
        App::new()
            .app_data(Data::new(server.app_state.clone()))
            .app_data(Data::new(server.senders.clone()))
            .app_data(Data::new(server.transaction_handler.clone()))
            .service(transact),
    )
    .await;

    let req = TestRequest::post()
        .uri("/transact")
        .set_json(TransactRequestData {
            contract_param: Default::default(),
            transaction_type: TransactionType::Withdraw,
            bridge_type: BridgeType::Loop,
            chain_id: 1,
            asset_symbol: "MTT".to_string(),
            asset_decimals: 16,
            pool_address: "0x4F416Acfd1153F9Af782056e68607227Af29D931".to_string(),
            circuit_type: CircuitType::Transaction1x0,
            signature: "0x800157ae47e94a156c42584190c33362b13ff94a7e8f5ef6ffd602c8d19ae\
        0684a4da6afd3c10bae9bd252dd20a9388d86c617bacb807a236a0285603e4086d61c"
                .to_string(),
        })
        .to_request();

    let resp: ApiResponse<RelayTransactResponse> = call_and_read_body_json(&app, req).await;
    assert_eq!(resp.code, ResponseCode::ChainIdNotFound as i32);
    assert!(resp.message.is_some());
    assert_eq!(
        resp.message.unwrap(),
        ResponseError::ChainIdNotFoundInRelayerConfig { chain_id: 1 }.to_string()
    );
}

#[actix_rt::test]
async fn test_send_transaction_to_queue_failed() {
    // create test server
    let mock_provider = MockProvider::new();
    let mut server = TestServer::new(Some(mock_provider.clone())).await.unwrap();

    let gas = U256::from(100_000_000_000u64);
    let nonce = U256::from(100);
    let price = U256::from(1000000);
    mock_provider.push(price).unwrap();
    mock_provider.push(gas).unwrap();
    mock_provider.push(nonce).unwrap();
    mock_provider.push(price).unwrap();

    let (senders, _) = transact_channel::init(
        &server.app_state.server_config,
        &server.app_state.relayer_config,
        &server.app_state.mystiko_config,
        server.providers.clone(),
        server.transaction_handler.clone(),
        server.token_price.clone(),
        1,
    )
    .await
    .unwrap();
    server.senders = senders;

    // init service
    let app = init_service(
        App::new()
            .app_data(Data::new(server.app_state.clone()))
            .app_data(Data::new(server.senders.clone()))
            .app_data(Data::new(server.transaction_handler.clone()))
            .service(transact)
            .service(transaction_status),
    )
    .await;

    drop(server.senders);

    let req = TestRequest::post()
        .uri("/transact")
        .set_json(get_valid_transact_request_data())
        .to_request();
    let resp: ApiResponse<RelayTransactResponse> = call_and_read_body_json(&app, req).await;
    assert_eq!(resp.code, ResponseCode::TransactionChannelError as i32);
}

#[actix_rt::test]
async fn test_unsupported_transaction() {
    // create test server
    let mock_provider = MockProvider::new();
    let server = TestServer::new(Some(mock_provider.clone())).await.unwrap();

    // init service
    let app = init_service(
        App::new()
            .app_data(Data::new(server.app_state.clone()))
            .app_data(Data::new(server.senders.clone()))
            .app_data(Data::new(server.transaction_handler.clone()))
            .service(transact)
            .service(transaction_status),
    )
    .await;

    let gas = U256::from(100_000_000_000u64);
    let nonce = U256::from(100);
    let price = U256::from(1000000);
    mock_provider.push(price).unwrap();
    mock_provider.push(gas).unwrap();
    mock_provider.push(nonce).unwrap();
    mock_provider.push(price).unwrap();

    let mut transact_data = get_valid_transact_request_data();
    transact_data.asset_symbol = "mtt1".to_string();

    let req = TestRequest::post()
        .uri("/transact")
        .set_json(transact_data)
        .to_request();
    let resp: ApiResponse<RelayTransactResponse> = call_and_read_body_json(&app, req).await;
    assert_eq!(resp.code, ResponseCode::UnsupportedTransaction as i32);
}
