use crate::common::{TestServer, TOKEN_PRICE_CONFIG_PATH};
use actix_web::test::{call_and_read_body_json, init_service, TestRequest};
use actix_web::web::Data;
use actix_web::App;
use ethereum_types::U256;
use ethers_providers::MockProvider;
use mockito::Matcher;
use mystiko_fs::read_file_bytes;
use mystiko_relayer::database::Database;
use mystiko_relayer::error::ResponseError;
use mystiko_relayer::handler::account::AccountHandler;
use mystiko_relayer::service::{info, minimum_gas_fee};
use mystiko_relayer_types::response::{ApiResponse, ResponseCode};
use mystiko_relayer_types::{RegisterInfoRequest, RegisterInfoResponse, RegisterOptions};
use mystiko_server_utils::token_price::query::CurrencyQuoteResponse;
use mystiko_storage::SqlStatementFormatter;
use mystiko_storage_sqlite::SqliteStorageBuilder;
use mystiko_types::CircuitType;
use std::sync::Arc;

#[actix_rt::test]
async fn test_no_option_successful() {
    // create test server
    let server = TestServer::singleton().await;
    // init service
    let app = init_service(
        App::new()
            .app_data(Data::new(server.app_state.clone()))
            .app_data(Data::new(server.account_handler.clone()))
            .app_data(Data::new(server.token_price.clone()))
            .app_data(Data::new(server.providers.clone()))
            .service(info),
    )
    .await;

    let chain_id = 5;
    let req = TestRequest::post()
        .uri("/info")
        .set_json(RegisterInfoRequest::builder().chain_id(chain_id).build())
        .to_request();
    let resp: ApiResponse<RegisterInfoResponse> = call_and_read_body_json(&app, req).await;
    let result = resp.data.unwrap();
    let contracts = result.contracts.unwrap();

    assert_eq!(resp.code, ResponseCode::Successful as i32);
    assert!(result.support);
    assert_eq!(result.available, Some(true));
    assert_eq!(result.chain_id, chain_id);
    assert_eq!(
        &result.relayer_contract_address.unwrap(),
        "0x45B22A8CefDfF00989882CAE48Ad06D57938Efcc"
    );
    assert_eq!(contracts.len(), 2);
    assert!(contracts.get(0).unwrap().minimum_gas_fee.is_none());
}

#[actix_rt::test]
async fn test_successful_with_options_erc20() {
    // create mock provider
    let mock_provider = MockProvider::new();
    // create test server
    let mut server = TestServer::new(Some(mock_provider.clone())).await.unwrap();
    // init service
    let app = init_service(
        App::new()
            .app_data(Data::new(server.app_state.clone()))
            .app_data(Data::new(server.account_handler.clone()))
            .app_data(Data::new(server.token_price.clone()))
            .app_data(Data::new(server.providers.clone()))
            .service(info),
    )
    .await;

    let id_bytes = read_file_bytes(TOKEN_PRICE_CONFIG_PATH).await.unwrap();
    let currency_quote: CurrencyQuoteResponse = serde_json::from_slice(&id_bytes).unwrap();
    let mock = server
        .mock_server
        .mock("GET", "/v2/cryptocurrency/quotes/latest")
        .expect(1)
        .match_query(Matcher::Any)
        .with_body(serde_json::to_string(&currency_quote).expect("Failed to serialize struct to JSON"))
        .create_async()
        .await;

    let gas_price = U256::from(1000000);
    mock_provider.push(gas_price).unwrap();

    let chain_id = 5;
    let asset_symbol = "MTT";
    let req = TestRequest::post()
        .uri("/info")
        .set_json(
            RegisterInfoRequest::builder()
                .chain_id(chain_id)
                .options(Some(
                    RegisterOptions::builder()
                        .asset_symbol(String::from(asset_symbol))
                        .circuit_type(CircuitType::Transaction1x0)
                        .show_unavailable(false)
                        .build(),
                ))
                .build(),
        )
        .to_request();
    let resp: ApiResponse<RegisterInfoResponse> = call_and_read_body_json(&app, req).await;
    mock.assert_async().await;

    let result = resp.data.unwrap();
    let contracts = result.contracts.unwrap();

    assert_eq!(resp.code, ResponseCode::Successful as i32);
    assert!(result.support);
    assert_eq!(result.available, Some(true));
    assert_eq!(result.chain_id, chain_id);
    assert_eq!(
        &result.relayer_contract_address.unwrap(),
        "0x45B22A8CefDfF00989882CAE48Ad06D57938Efcc"
    );
    assert_eq!(contracts.len(), 1);
    assert_eq!(contracts.get(0).unwrap().asset_symbol, asset_symbol);
    assert_eq!(contracts.get(0).unwrap().relayer_fee_of_ten_thousandth, 25);
}

#[actix_rt::test]
async fn test_successful_with_options_main() {
    // create mock provider
    let mock_provider = MockProvider::new();
    // create test server
    let server = TestServer::new(Some(mock_provider.clone())).await.unwrap();
    // init service
    let app = init_service(
        App::new()
            .app_data(Data::new(server.app_state.clone()))
            .app_data(Data::new(server.account_handler.clone()))
            .app_data(Data::new(server.token_price.clone()))
            .app_data(Data::new(server.providers.clone()))
            .service(info),
    )
    .await;

    let gas_price = U256::from(1000000);
    mock_provider.push(gas_price).unwrap();

    let chain_id = 5;
    let asset_symbol = "ETH";
    let req = TestRequest::post()
        .uri("/info")
        .set_json(
            RegisterInfoRequest::builder()
                .chain_id(chain_id)
                .options(Some(
                    RegisterOptions::builder()
                        .asset_symbol(String::from(asset_symbol))
                        .circuit_type(CircuitType::Transaction1x0)
                        .show_unavailable(false)
                        .build(),
                ))
                .build(),
        )
        .to_request();
    let resp: ApiResponse<RegisterInfoResponse> = call_and_read_body_json(&app, req).await;
    let result = resp.data.unwrap();
    let contracts = result.contracts.unwrap();

    assert_eq!(resp.code, ResponseCode::Successful as i32);
    assert!(result.support);
    assert_eq!(result.available, Some(true));
    assert_eq!(result.chain_id, chain_id);
    assert_eq!(
        &result.relayer_contract_address.unwrap(),
        "0x45B22A8CefDfF00989882CAE48Ad06D57938Efcc"
    );
    assert_eq!(contracts.len(), 1);
    assert_eq!(contracts.get(0).unwrap().asset_symbol, asset_symbol);
    assert_eq!(contracts.get(0).unwrap().relayer_fee_of_ten_thousandth, 25);
}

#[actix_rt::test]
async fn test_unsupported_asset_symbol() {
    let server = TestServer::singleton().await;
    // init service
    let app = init_service(
        App::new()
            .app_data(Data::new(server.app_state.clone()))
            .app_data(Data::new(server.account_handler.clone()))
            .app_data(Data::new(server.token_price.clone()))
            .app_data(Data::new(server.providers.clone()))
            .service(info),
    )
    .await;

    let chain_id = 5;
    let asset_symbol = "UNSUPPORTED";
    let req = TestRequest::post()
        .uri("/info")
        .set_json(
            RegisterInfoRequest::builder()
                .chain_id(chain_id)
                .options(Some(
                    RegisterOptions::builder()
                        .asset_symbol(String::from(asset_symbol))
                        .circuit_type(CircuitType::Transaction1x0)
                        .show_unavailable(false)
                        .build(),
                ))
                .build(),
        )
        .to_request();

    let resp: ApiResponse<RegisterInfoResponse> = call_and_read_body_json(&app, req).await;
    let result = resp.data.unwrap();
    assert_eq!(resp.code, ResponseCode::Successful as i32);
    assert!(!result.support);
    assert_eq!(result.available, Some(false));
    assert_eq!(result.chain_id, chain_id);
    assert!(&result.relayer_contract_address.is_none());
    assert!(&result.contracts.is_none());
}

#[actix_rt::test]
async fn test_relayer_unavailable() {
    let server = TestServer::singleton().await;
    // init service
    let app = init_service(
        App::new()
            .app_data(Data::new(server.app_state.clone()))
            .app_data(Data::new(server.account_handler.clone()))
            .app_data(Data::new(server.token_price.clone()))
            .app_data(Data::new(server.providers.clone()))
            .service(info),
    )
    .await;

    let chain_id = 97;
    let asset_symbol = "mUSD";
    let req = TestRequest::post()
        .uri("/info")
        .set_json(
            RegisterInfoRequest::builder()
                .chain_id(chain_id)
                .options(Some(
                    RegisterOptions::builder()
                        .asset_symbol(String::from(asset_symbol))
                        .circuit_type(CircuitType::Transaction1x0)
                        .show_unavailable(true)
                        .build(),
                ))
                .build(),
        )
        .to_request();

    let resp: ApiResponse<RegisterInfoResponse> = call_and_read_body_json(&app, req).await;
    let result = resp.data.unwrap();
    assert_eq!(resp.code, ResponseCode::Successful as i32);
    assert!(result.support);
    assert_eq!(result.available, Some(false));
    assert_eq!(result.chain_id, chain_id);
    assert!(&result.relayer_contract_address.is_none());
    assert!(&result.contracts.is_none());
}

#[actix_rt::test]
async fn test_chain_id_not_found() {
    let server = TestServer::singleton().await;
    let result = minimum_gas_fee(
        server.app_state.relayer_config.as_ref(),
        9999,
        U256::zero(),
        Data::new(server.token_price.clone()),
        &RegisterOptions {
            asset_symbol: "MTT".to_string(),
            circuit_type: CircuitType::Transaction1x0,
            show_unavailable: false,
        },
    )
    .await;

    assert!(result.is_err());
    assert_eq!(
        &result.unwrap_err().to_string(),
        "chain id 9999 config not found in relayer config"
    );
}

#[actix_rt::test]
async fn test_contract_config_not_found() {
    let server = TestServer::singleton().await;
    let result = minimum_gas_fee(
        server.app_state.relayer_config.as_ref(),
        5,
        U256::zero(),
        Data::new(server.token_price.clone()),
        &RegisterOptions {
            asset_symbol: "UST".to_string(),
            circuit_type: CircuitType::Transaction1x0,
            show_unavailable: false,
        },
    )
    .await;

    assert!(result.is_err());
    assert_eq!(
        &result.unwrap_err().to_string(),
        "asset symbol UST contract config not found in chain id 5 config"
    );
}

#[actix_rt::test]
async fn test_not_supported_chain_id() {
    // create test server
    let server = TestServer::singleton().await;
    // init service
    let app = init_service(
        App::new()
            .app_data(Data::new(server.app_state.clone()))
            .app_data(Data::new(server.account_handler.clone()))
            .app_data(Data::new(server.token_price.clone()))
            .app_data(Data::new(server.providers.clone()))
            .service(info),
    )
    .await;

    let chain_id = 51111;
    let req = TestRequest::post()
        .uri("/info")
        .set_json(RegisterInfoRequest::builder().chain_id(chain_id).build())
        .to_request();
    let resp: ApiResponse<RegisterInfoResponse> = call_and_read_body_json(&app, req).await;
    let result = resp.data.unwrap();
    assert_eq!(resp.code, ResponseCode::Successful as i32);
    assert!(!result.support);
    assert_eq!(result.available, Some(false));
}

#[actix_rt::test]
async fn test_gas_price_error() {
    // create test server
    let server = TestServer::singleton().await;
    // init service
    let app = init_service(
        App::new()
            .app_data(Data::new(server.app_state.clone()))
            .app_data(Data::new(server.account_handler.clone()))
            .app_data(Data::new(server.token_price.clone()))
            .app_data(Data::new(server.providers.clone()))
            .service(info),
    )
    .await;

    let chain_id = 5;
    let options = RegisterOptions {
        asset_symbol: "mtt".to_string(),
        circuit_type: CircuitType::Transaction1x0,
        show_unavailable: true,
    };
    let req = TestRequest::post()
        .uri("/info")
        .set_json(
            RegisterInfoRequest::builder()
                .chain_id(chain_id)
                .options(Some(options))
                .build(),
        )
        .to_request();
    let resp: ApiResponse<RegisterInfoResponse> = call_and_read_body_json(&app, req).await;

    assert_eq!(resp.code, ResponseCode::GetGasPriceError as i32);
    assert!(resp.data.is_none());
    assert!(resp.message.is_some());
    assert_eq!(
        resp.message.unwrap(),
        ResponseError::GetGasPriceError { chain_id: 5 }.to_string()
    );
}

#[actix_rt::test]
async fn test_minimum_gas_fee_error() {
    // create mock provider
    let mock_provider = MockProvider::new();
    // create test server
    let server = TestServer::new(Some(mock_provider.clone())).await.unwrap();
    // init service
    let app = init_service(
        App::new()
            .app_data(Data::new(server.app_state.clone()))
            .app_data(Data::new(server.account_handler.clone()))
            .app_data(Data::new(server.token_price.clone()))
            .app_data(Data::new(server.providers.clone()))
            .service(info),
    )
    .await;

    let gas_price = U256::from(1000000);
    mock_provider.push(gas_price).unwrap();

    let chain_id = 5;
    let asset_symbol = "MTT";
    let req = TestRequest::post()
        .uri("/info")
        .set_json(
            RegisterInfoRequest::builder()
                .chain_id(chain_id)
                .options(Some(
                    RegisterOptions::builder()
                        .asset_symbol(String::from(asset_symbol))
                        .circuit_type(CircuitType::Transaction1x0)
                        .show_unavailable(false)
                        .build(),
                ))
                .build(),
        )
        .to_request();
    let resp: ApiResponse<RegisterInfoResponse> = call_and_read_body_json(&app, req).await;
    assert_eq!(resp.code, ResponseCode::GetMinimumGasFeeFailed as i32);
}

#[actix_rt::test]
async fn test_account_not_found_in_db() {
    // create test server
    let mut server = TestServer::new(None).await.unwrap();
    let database = Database::new(
        SqlStatementFormatter::sqlite(),
        SqliteStorageBuilder::new().in_memory().build().await.unwrap(),
    );
    database.migrate().await.unwrap();
    server.account_handler = Arc::new(AccountHandler::new(Arc::new(database), &[]).await.unwrap());

    // init service
    let app = init_service(
        App::new()
            .app_data(Data::new(server.app_state.clone()))
            .app_data(Data::new(server.account_handler.clone()))
            .app_data(Data::new(server.token_price.clone()))
            .app_data(Data::new(server.providers.clone()))
            .service(info),
    )
    .await;

    let chain_id = 5;
    let req = TestRequest::post()
        .uri("/info")
        .set_json(RegisterInfoRequest::builder().chain_id(chain_id).build())
        .to_request();

    let resp: ApiResponse<RegisterInfoResponse> = call_and_read_body_json(&app, req).await;
    assert_eq!(resp.code, ResponseCode::AccountNotFoundInDatabase as i32);
}
