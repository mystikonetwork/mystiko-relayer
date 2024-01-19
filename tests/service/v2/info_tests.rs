use crate::channel::{MockConsumers, MockProducers};
use crate::common::MockTokenPrice;
use crate::handler::{MockAccounts, MockTransactions};
use crate::service::{create_app, MockOptions, MockProvider};
use actix_web::test::{call_and_read_body_json, TestRequest};
use ethereum_types::U256;
use mystiko_relayer::database::account::Account;
use mystiko_relayer::error::RelayerServerError;
use mystiko_relayer_types::response::{ApiResponse, ResponseCode};
use mystiko_relayer_types::{RegisterInfoRequest, RegisterInfoResponse, RegisterOptions};
use mystiko_server_utils::token_price::PriceMiddlewareError;
use mystiko_storage::{Document, StorageError};
use mystiko_types::CircuitType;
use std::collections::HashMap;

const CHAIN_ID: u64 = 5;

#[actix_rt::test]
async fn test_success() {
    let mut account_handler = MockAccounts::new();
    account_handler
        .expect_find_by_chain_id()
        .withf(|chain_id| chain_id == &CHAIN_ID)
        .returning(|chain_id| {
            Ok(vec![Document::new(
                "123456".to_string(),
                1234567890u64,
                1234567891u64,
                Account {
                    chain_address: "0x1234567890".to_string(),
                    chain_id,
                    available: true,
                    supported_erc20_tokens: vec!["mtt".to_string()],
                    balance_alarm_threshold: 0.0,
                    balance_check_interval_ms: 0,
                    insufficient_balances: false,
                },
            )])
        });
    let token_price = MockTokenPrice::new();
    let provider = MockProvider::builder()
        .base_fee_per_gas(U256::from(100000))
        .max_fee_per_gas(U256::from(1000000))
        .priority_fee(U256::from(10000))
        .build();

    let mut providers = HashMap::new();
    providers.insert(CHAIN_ID, provider);
    let options = MockOptions {
        chain_id: CHAIN_ID,
        providers,
        transaction_handler: MockTransactions::new(),
        account_handler,
        token_price,
        consumer: MockConsumers::new(),
        producer: MockProducers::new(),
    };
    let app = create_app(options).await.unwrap();

    let request = TestRequest::post()
        .uri("/api/v2/info")
        .set_json(RegisterInfoRequest::builder().chain_id(CHAIN_ID).build())
        .to_request();
    let response: ApiResponse<RegisterInfoResponse> = call_and_read_body_json(&app, request).await;
    assert_eq!(response.code, ResponseCode::Successful as i32);
    assert!(response.data.is_some());
    let data = response.data.unwrap();
    assert!(data.support);
    assert!(data.available.is_some());
    assert!(data.available.unwrap());
    assert!(data.contracts.is_some());
}

#[actix_rt::test]
async fn test_success_with_options_erc20() {
    let mut account_handler = MockAccounts::new();
    let mut token_price = MockTokenPrice::new();
    account_handler
        .expect_find_by_chain_id()
        .withf(|chain_id| chain_id == &CHAIN_ID)
        .returning(|chain_id| {
            Ok(vec![Document::new(
                "123456".to_string(),
                1234567890u64,
                1234567891u64,
                Account {
                    chain_address: "0x1234567890".to_string(),
                    chain_id,
                    available: true,
                    supported_erc20_tokens: vec!["mtt".to_string()],
                    balance_alarm_threshold: 0.0,
                    balance_check_interval_ms: 0,
                    insufficient_balances: false,
                },
            )])
        });
    token_price
        .expect_swap()
        .withf(|asset_a, _, _, asset_b, _| asset_a == "ETH" && asset_b == "mtt")
        .returning(|_, _, _, _, _| Ok(U256::from(10000)));
    let provider = MockProvider::builder()
        .base_fee_per_gas(U256::from(100000))
        .max_fee_per_gas(U256::from(1000000))
        .priority_fee(U256::from(10000))
        .gas_price(U256::from(10000))
        .build();

    let mut providers = HashMap::new();
    providers.insert(CHAIN_ID, provider);
    let options = MockOptions {
        chain_id: CHAIN_ID,
        providers,
        transaction_handler: MockTransactions::new(),
        account_handler,
        token_price,
        consumer: MockConsumers::new(),
        producer: MockProducers::new(),
    };
    let app = create_app(options).await.unwrap();

    let request = TestRequest::post()
        .uri("/api/v2/info")
        .set_json(
            RegisterInfoRequest::builder()
                .chain_id(CHAIN_ID)
                .options(
                    RegisterOptions::builder()
                        .asset_symbol("mtt")
                        .circuit_type(CircuitType::Transaction1x0)
                        .show_unavailable(false)
                        .build(),
                )
                .build(),
        )
        .to_request();
    let response: ApiResponse<RegisterInfoResponse> = call_and_read_body_json(&app, request).await;
    assert_eq!(response.code, ResponseCode::Successful as i32);
    assert!(response.data.is_some());
    let data = response.data.unwrap();
    assert_eq!(data.chain_id, CHAIN_ID);
    assert!(data.support);
    assert!(data.available.is_some());
    assert!(data.available.unwrap());
    let contracts = data.contracts.unwrap();
    assert_eq!(contracts.len(), 1);
    assert_eq!(contracts[0].asset_symbol, "MTT");
    assert_eq!(contracts[0].relayer_fee_of_ten_thousandth, 25);
}

#[actix_rt::test]
async fn test_success_with_options_main() {
    let mut account_handler = MockAccounts::new();
    let mut token_price = MockTokenPrice::new();
    account_handler
        .expect_find_by_chain_id()
        .withf(|chain_id| chain_id == &CHAIN_ID)
        .returning(|chain_id| {
            Ok(vec![Document::new(
                "123456".to_string(),
                1234567890u64,
                1234567891u64,
                Account {
                    chain_address: "0x1234567890".to_string(),
                    chain_id,
                    available: true,
                    supported_erc20_tokens: vec!["mtt".to_string()],
                    balance_alarm_threshold: 0.0,
                    balance_check_interval_ms: 0,
                    insufficient_balances: false,
                },
            )])
        });
    token_price
        .expect_swap()
        .withf(|asset_a, _, _, asset_b, _| asset_a == "ETH" && asset_b == "ETH")
        .returning(|_, _, _, _, _| Ok(U256::from(10000)));
    let provider = MockProvider::builder()
        .base_fee_per_gas(U256::from(100000))
        .max_fee_per_gas(U256::from(1000000))
        .priority_fee(U256::from(10000))
        .gas_price(U256::from(10000))
        .build();

    let mut providers = HashMap::new();
    providers.insert(CHAIN_ID, provider);
    let options = MockOptions {
        chain_id: CHAIN_ID,
        providers,
        transaction_handler: MockTransactions::new(),
        account_handler,
        token_price,
        consumer: MockConsumers::new(),
        producer: MockProducers::new(),
    };
    let app = create_app(options).await.unwrap();

    let request = TestRequest::post()
        .uri("/api/v2/info")
        .set_json(
            RegisterInfoRequest::builder()
                .chain_id(CHAIN_ID)
                .options(
                    RegisterOptions::builder()
                        .asset_symbol("eth")
                        .circuit_type(CircuitType::Transaction1x0)
                        .show_unavailable(false)
                        .build(),
                )
                .build(),
        )
        .to_request();
    let response: ApiResponse<RegisterInfoResponse> = call_and_read_body_json(&app, request).await;
    assert_eq!(response.code, ResponseCode::Successful as i32);
    assert!(response.data.is_some());
    let data = response.data.unwrap();
    assert_eq!(data.chain_id, CHAIN_ID);
    assert!(data.support);
    assert!(data.available.is_some());
    assert!(data.available.unwrap());
    let contracts = data.contracts.unwrap();
    assert_eq!(contracts.len(), 1);
    assert_eq!(contracts[0].asset_symbol, "ETH");
    assert_eq!(contracts[0].relayer_fee_of_ten_thousandth, 25);
}

#[actix_rt::test]
async fn test_with_mystiko_chain_config_not_found() {
    let options = MockOptions {
        chain_id: CHAIN_ID,
        providers: HashMap::new(),
        transaction_handler: MockTransactions::new(),
        account_handler: MockAccounts::new(),
        token_price: MockTokenPrice::new(),
        consumer: MockConsumers::new(),
        producer: MockProducers::new(),
    };
    let app = create_app(options).await.unwrap();

    let request = TestRequest::post()
        .uri("/api/v2/info")
        .set_json(
            RegisterInfoRequest::builder()
                .chain_id(1u64)
                .options(
                    RegisterOptions::builder()
                        .asset_symbol("eth")
                        .circuit_type(CircuitType::Transaction1x0)
                        .show_unavailable(false)
                        .build(),
                )
                .build(),
        )
        .to_request();
    let response: ApiResponse<RegisterInfoResponse> = call_and_read_body_json(&app, request).await;
    assert_eq!(response.code, ResponseCode::Successful as i32);
    let data = &response.data.unwrap();
    assert_eq!(data.chain_id, 1);
    assert!(!data.support);
    assert!(data.available.is_some());
    assert!(!&data.available.unwrap());
}

#[actix_rt::test]
async fn test_with_relayer_chain_config_not_found() {
    let options = MockOptions {
        chain_id: CHAIN_ID,
        providers: HashMap::new(),
        transaction_handler: MockTransactions::new(),
        account_handler: MockAccounts::new(),
        token_price: MockTokenPrice::new(),
        consumer: MockConsumers::new(),
        producer: MockProducers::new(),
    };
    let app = create_app(options).await.unwrap();

    let request = TestRequest::post()
        .uri("/api/v2/info")
        .set_json(
            RegisterInfoRequest::builder()
                .chain_id(80001u64)
                .options(
                    RegisterOptions::builder()
                        .asset_symbol("eth")
                        .circuit_type(CircuitType::Transaction1x0)
                        .show_unavailable(false)
                        .build(),
                )
                .build(),
        )
        .to_request();
    let response: ApiResponse<RegisterInfoResponse> = call_and_read_body_json(&app, request).await;
    assert_eq!(response.code, ResponseCode::Successful as i32);
    let data = &response.data.unwrap();
    assert_eq!(data.chain_id, 80001);
    assert!(!data.support);
    assert!(data.available.is_some());
    assert!(!&data.available.unwrap());
}

#[actix_rt::test]
async fn test_with_gas_price_error() {
    let mut account_handler = MockAccounts::new();
    account_handler
        .expect_find_by_chain_id()
        .withf(|chain_id| chain_id == &CHAIN_ID)
        .returning(|chain_id| {
            Ok(vec![Document::new(
                "123456".to_string(),
                1234567890u64,
                1234567891u64,
                Account {
                    chain_address: "0x1234567890".to_string(),
                    chain_id,
                    available: true,
                    supported_erc20_tokens: vec!["mtt".to_string()],
                    balance_alarm_threshold: 0.0,
                    balance_check_interval_ms: 0,
                    insufficient_balances: false,
                },
            )])
        });

    let options = MockOptions {
        chain_id: CHAIN_ID,
        providers: HashMap::new(),
        transaction_handler: MockTransactions::new(),
        account_handler,
        token_price: MockTokenPrice::new(),
        consumer: MockConsumers::new(),
        producer: MockProducers::new(),
    };
    let app = create_app(options).await.unwrap();

    let request = TestRequest::post()
        .uri("/api/v2/info")
        .set_json(
            RegisterInfoRequest::builder()
                .chain_id(CHAIN_ID)
                .options(
                    RegisterOptions::builder()
                        .asset_symbol("mtt")
                        .circuit_type(CircuitType::Transaction1x0)
                        .show_unavailable(false)
                        .build(),
                )
                .build(),
        )
        .to_request();
    let response: ApiResponse<RegisterInfoResponse> = call_and_read_body_json(&app, request).await;
    assert_eq!(response.code, ResponseCode::GetGasPriceError as i32);
    assert!(response.data.is_none());
    assert!(response.message.is_some());
}

#[actix_rt::test]
async fn test_with_database_error() {
    let mut account_handler = MockAccounts::new();
    account_handler
        .expect_find_by_chain_id()
        .withf(|chain_id| chain_id == &CHAIN_ID)
        .returning(|_| {
            Err(RelayerServerError::StorageError(StorageError::NoSuchColumnError(
                "mock test".to_string(),
            )))
        });

    let options = MockOptions {
        chain_id: CHAIN_ID,
        providers: HashMap::new(),
        transaction_handler: MockTransactions::new(),
        account_handler,
        token_price: MockTokenPrice::new(),
        consumer: MockConsumers::new(),
        producer: MockProducers::new(),
    };
    let app = create_app(options).await.unwrap();

    let request = TestRequest::post()
        .uri("/api/v2/info")
        .set_json(RegisterInfoRequest::builder().chain_id(CHAIN_ID).build())
        .to_request();
    let response: ApiResponse<RegisterInfoResponse> = call_and_read_body_json(&app, request).await;
    assert_eq!(response.code, ResponseCode::DatabaseError as i32);
}

#[actix_rt::test]
async fn test_with_account_empty() {
    let mut account_handler = MockAccounts::new();
    account_handler
        .expect_find_by_chain_id()
        .withf(|chain_id| chain_id == &CHAIN_ID)
        .returning(|_| Ok(vec![]));

    let options = MockOptions {
        chain_id: CHAIN_ID,
        providers: HashMap::new(),
        transaction_handler: MockTransactions::new(),
        account_handler,
        token_price: MockTokenPrice::new(),
        consumer: MockConsumers::new(),
        producer: MockProducers::new(),
    };
    let app = create_app(options).await.unwrap();

    let request = TestRequest::post()
        .uri("/api/v2/info")
        .set_json(RegisterInfoRequest::builder().chain_id(CHAIN_ID).build())
        .to_request();
    let response: ApiResponse<RegisterInfoResponse> = call_and_read_body_json(&app, request).await;
    assert_eq!(response.code, ResponseCode::AccountNotFoundInDatabase as i32);
}

#[actix_rt::test]
async fn test_with_not_supported_asset_symbol() {
    let mut account_handler = MockAccounts::new();
    let mut token_price = MockTokenPrice::new();
    account_handler
        .expect_find_by_chain_id()
        .withf(|chain_id| chain_id == &CHAIN_ID)
        .returning(|chain_id| {
            Ok(vec![Document::new(
                "123456".to_string(),
                1234567890u64,
                1234567891u64,
                Account {
                    chain_address: "0x1234567890".to_string(),
                    chain_id,
                    available: true,
                    supported_erc20_tokens: vec!["mtt".to_string()],
                    balance_alarm_threshold: 0.0,
                    balance_check_interval_ms: 0,
                    insufficient_balances: false,
                },
            )])
        });
    token_price
        .expect_swap()
        .withf(|asset_a, _, _, asset_b, _| asset_a == "ETH" && asset_b == "mtt")
        .returning(|_, _, _, _, _| Ok(U256::from(10000)));
    let provider = MockProvider::builder()
        .base_fee_per_gas(U256::from(100000))
        .max_fee_per_gas(U256::from(1000000))
        .priority_fee(U256::from(10000))
        .gas_price(U256::from(10000))
        .build();

    let mut providers = HashMap::new();
    providers.insert(CHAIN_ID, provider);
    let options = MockOptions {
        chain_id: CHAIN_ID,
        providers,
        transaction_handler: MockTransactions::new(),
        account_handler,
        token_price,
        consumer: MockConsumers::new(),
        producer: MockProducers::new(),
    };
    let app = create_app(options).await.unwrap();

    let request = TestRequest::post()
        .uri("/api/v2/info")
        .set_json(
            RegisterInfoRequest::builder()
                .chain_id(CHAIN_ID)
                .options(
                    RegisterOptions::builder()
                        .asset_symbol("mtt1")
                        .circuit_type(CircuitType::Transaction1x0)
                        .show_unavailable(false)
                        .build(),
                )
                .build(),
        )
        .to_request();
    let response: ApiResponse<RegisterInfoResponse> = call_and_read_body_json(&app, request).await;
    assert_eq!(response.code, ResponseCode::Successful as i32);
    let data = &response.data.unwrap();
    assert_eq!(data.chain_id, CHAIN_ID);
    assert!(!data.support);
    assert!(!&data.available.unwrap());
}

#[actix_rt::test]
async fn test_with_not_available() {
    let mut account_handler = MockAccounts::new();
    let mut token_price = MockTokenPrice::new();
    account_handler
        .expect_find_by_chain_id()
        .withf(|chain_id| chain_id == &CHAIN_ID)
        .returning(|chain_id| {
            Ok(vec![Document::new(
                "123456".to_string(),
                1234567890u64,
                1234567891u64,
                Account {
                    chain_address: "0x1234567890".to_string(),
                    chain_id,
                    available: false,
                    supported_erc20_tokens: vec!["mtt".to_string()],
                    balance_alarm_threshold: 0.0,
                    balance_check_interval_ms: 0,
                    insufficient_balances: false,
                },
            )])
        });
    token_price
        .expect_swap()
        .withf(|asset_a, _, _, asset_b, _| asset_a == "ETH" && asset_b == "mtt")
        .returning(|_, _, _, _, _| Ok(U256::from(10000)));
    let provider = MockProvider::builder()
        .base_fee_per_gas(U256::from(100000))
        .max_fee_per_gas(U256::from(1000000))
        .priority_fee(U256::from(10000))
        .gas_price(U256::from(10000))
        .build();

    let mut providers = HashMap::new();
    providers.insert(CHAIN_ID, provider);
    let options = MockOptions {
        chain_id: CHAIN_ID,
        providers,
        transaction_handler: MockTransactions::new(),
        account_handler,
        token_price,
        consumer: MockConsumers::new(),
        producer: MockProducers::new(),
    };
    let app = create_app(options).await.unwrap();

    let request = TestRequest::post()
        .uri("/api/v2/info")
        .set_json(
            RegisterInfoRequest::builder()
                .chain_id(CHAIN_ID)
                .options(
                    RegisterOptions::builder()
                        .asset_symbol("mtt")
                        .circuit_type(CircuitType::Transaction1x0)
                        .show_unavailable(false)
                        .build(),
                )
                .build(),
        )
        .to_request();
    let response: ApiResponse<RegisterInfoResponse> = call_and_read_body_json(&app, request).await;
    assert_eq!(response.code, ResponseCode::Successful as i32);
    let data = &response.data.unwrap();
    assert!(data.support);
    assert!(!&data.available.unwrap());
}

#[actix_rt::test]
async fn test_with_minimum_gas_fee_error() {
    let mut account_handler = MockAccounts::new();
    let mut token_price = MockTokenPrice::new();
    account_handler
        .expect_find_by_chain_id()
        .withf(|chain_id| chain_id == &CHAIN_ID)
        .returning(|chain_id| {
            Ok(vec![Document::new(
                "123456".to_string(),
                1234567890u64,
                1234567891u64,
                Account {
                    chain_address: "0x1234567890".to_string(),
                    chain_id,
                    available: true,
                    supported_erc20_tokens: vec!["mtt".to_string()],
                    balance_alarm_threshold: 0.0,
                    balance_check_interval_ms: 0,
                    insufficient_balances: false,
                },
            )])
        });
    token_price
        .expect_swap()
        .withf(|asset_a, _, _, asset_b, _| asset_a == "ETH" && asset_b == "mtt")
        .returning(|_, _, _, _, _| Err(PriceMiddlewareError::InternalError));
    let provider = MockProvider::builder()
        .base_fee_per_gas(U256::from(100000))
        .max_fee_per_gas(U256::from(1000000))
        .priority_fee(U256::from(10000))
        .gas_price(U256::from(10000))
        .build();

    let mut providers = HashMap::new();
    providers.insert(CHAIN_ID, provider);
    let options = MockOptions {
        chain_id: CHAIN_ID,
        providers,
        transaction_handler: MockTransactions::new(),
        account_handler,
        token_price,
        consumer: MockConsumers::new(),
        producer: MockProducers::new(),
    };
    let app = create_app(options).await.unwrap();

    let request = TestRequest::post()
        .uri("/api/v2/info")
        .set_json(
            RegisterInfoRequest::builder()
                .chain_id(CHAIN_ID)
                .options(
                    RegisterOptions::builder()
                        .asset_symbol("mtt")
                        .circuit_type(CircuitType::Transaction1x0)
                        .show_unavailable(false)
                        .build(),
                )
                .build(),
        )
        .to_request();
    let response: ApiResponse<RegisterInfoResponse> = call_and_read_body_json(&app, request).await;
    assert_eq!(response.code, ResponseCode::GetMinimumGasFeeFailed as i32);
}
