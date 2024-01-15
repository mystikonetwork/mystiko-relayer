use crate::channel::MockConsumers;
use crate::common::MockTokenPrice;
use crate::handler::{MockAccounts, MockTransactions};
use crate::service::{create_app, MockOptions};
use actix_web::test::{call_and_read_body_json, TestRequest};
use mystiko_relayer::database::account::Account;
use mystiko_relayer::service::v1::response::ChainStatusResponse;
use mystiko_relayer_types::response::ApiResponse;
use mystiko_storage::Document;
use std::collections::HashMap;

const CHAIN_ID: u64 = 5;

#[actix_rt::test]
async fn test_success() {
    let mut account_handler = MockAccounts::new();
    let token_price = MockTokenPrice::new();
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
        providers: HashMap::new(),
        transaction_handler: MockTransactions::new(),
        account_handler,
        token_price,
        consumer: MockConsumers::new(),
    };
    let app = create_app(options).await.unwrap();

    let request = TestRequest::get().uri("/status").to_request();
    let response: ApiResponse<ChainStatusResponse> = call_and_read_body_json(&app, request).await;
}

#[actix_rt::test]
async fn test_success_with_options_erc20() {}

#[actix_rt::test]
async fn test_success_with_options_main() {}
