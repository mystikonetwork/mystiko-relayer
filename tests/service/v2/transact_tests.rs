use crate::channel::{MockConsumers, MockProducers};
use crate::common::{default_transaction, MockTokenPrice};
use crate::handler::{MockAccounts, MockTransactions};
use crate::service::{create_app, MockOptions};
use actix_web::test::{call_and_read_body_json, TestRequest};
use anyhow::anyhow;
use ethereum_types::U256;
use ethers_core::types::Bytes;
use mystiko_abi::commitment_pool::TransactRequest;
use mystiko_protos::core::v1::SpendType;
use mystiko_relayer::error::RelayerServerError;
use mystiko_relayer_types::response::{ApiResponse, ResponseCode};
use mystiko_relayer_types::{RelayTransactResponse, TransactRequestData, TransactStatus};
use mystiko_storage::{Document, StorageError};
use mystiko_types::{BridgeType, CircuitType};
use std::collections::HashMap;
use std::str::FromStr;

const CHAIN_ID: u64 = 5;

#[actix_rt::test]
async fn test_success() {
    let data = transact_request_data();
    let signature = data.signature.clone();
    let mut transaction_handler = MockTransactions::new();
    transaction_handler
        .expect_is_repeated_transaction()
        .withf(move |sig| sig.eq(&signature))
        .returning(|_| Ok(false));
    transaction_handler
        .expect_find_by_id()
        .withf(|id| id == "123456")
        .returning(|_| {
            let mut transaction = default_transaction();
            transaction.status = TransactStatus::Succeeded;
            Ok(Some(Document::new(
                "123456".to_string(),
                1234567890u64,
                1234567891u64,
                transaction,
            )))
        });
    let mut producer = MockProducers::new();
    producer
        .expect_send()
        .withf(|data| data.chain_id == CHAIN_ID)
        .returning(|_| {
            Ok(Document::new(
                "123456".to_string(),
                1234567890u64,
                1234567891u64,
                default_transaction(),
            ))
        });
    let options = MockOptions {
        chain_id: CHAIN_ID,
        providers: HashMap::new(),
        transaction_handler,
        account_handler: MockAccounts::new(),
        token_price: MockTokenPrice::new(),
        consumer: MockConsumers::new(),
        producer,
    };
    let app = create_app(options).await.unwrap();

    let request = TestRequest::post().uri("/api/v2/transact").set_json(data).to_request();
    let response: ApiResponse<RelayTransactResponse> = call_and_read_body_json(&app, request).await;
    assert_eq!(response.code, ResponseCode::Successful as i32);
}

#[actix_rt::test]
async fn test_main_success() {
    let mut data = transact_request_data();
    data.asset_symbol = "ETH".to_string();
    let signature = data.signature.clone();
    let mut transaction_handler = MockTransactions::new();
    transaction_handler
        .expect_is_repeated_transaction()
        .withf(move |sig| sig.eq(&signature))
        .returning(|_| Ok(false));
    transaction_handler
        .expect_find_by_id()
        .withf(|id| id == "123456")
        .returning(|_| {
            let mut transaction = default_transaction();
            transaction.status = TransactStatus::Succeeded;
            Ok(Some(Document::new(
                "123456".to_string(),
                1234567890u64,
                1234567891u64,
                transaction,
            )))
        });
    let mut producer = MockProducers::new();
    producer
        .expect_send()
        .withf(|data| data.chain_id == CHAIN_ID)
        .returning(|_| {
            Ok(Document::new(
                "123456".to_string(),
                1234567890u64,
                1234567891u64,
                default_transaction(),
            ))
        });
    let options = MockOptions {
        chain_id: CHAIN_ID,
        providers: HashMap::new(),
        transaction_handler,
        account_handler: MockAccounts::new(),
        token_price: MockTokenPrice::new(),
        consumer: MockConsumers::new(),
        producer,
    };
    let app = create_app(options).await.unwrap();

    let request = TestRequest::post().uri("/api/v2/transact").set_json(data).to_request();
    let response: ApiResponse<RelayTransactResponse> = call_and_read_body_json(&app, request).await;
    assert_eq!(response.code, ResponseCode::Successful as i32);
}

#[actix_rt::test]
async fn test_with_invalid_request_data() {
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
    let mut data = transact_request_data();
    data.chain_id = 0u64;

    let request = TestRequest::post().uri("/api/v2/transact").set_json(data).to_request();
    let response: ApiResponse<RelayTransactResponse> = call_and_read_body_json(&app, request).await;
    assert_eq!(response.code, ResponseCode::ValidateError as i32);
    assert!(response.message.is_some());
    assert!(response.data.is_none());
}

#[actix_rt::test]
async fn test_with_repeated_transaction() {
    let data = transact_request_data();
    let signature = data.signature.clone();
    let mut transaction_handler = MockTransactions::new();
    transaction_handler
        .expect_is_repeated_transaction()
        .withf(move |sig| sig.eq(&signature))
        .returning(|_| Ok(true));
    let options = MockOptions {
        chain_id: CHAIN_ID,
        providers: HashMap::new(),
        transaction_handler,
        account_handler: MockAccounts::new(),
        token_price: MockTokenPrice::new(),
        consumer: MockConsumers::new(),
        producer: MockProducers::new(),
    };
    let app = create_app(options).await.unwrap();

    let request = TestRequest::post().uri("/api/v2/transact").set_json(data).to_request();
    let response: ApiResponse<RelayTransactResponse> = call_and_read_body_json(&app, request).await;
    assert_eq!(response.code, ResponseCode::RepeatedTransaction as i32);
    assert!(response.data.is_none());
    assert!(response.message.is_some());
}

#[actix_rt::test]
async fn test_with_database_error() {
    let data = transact_request_data();
    let signature = data.signature.clone();
    let mut transaction_handler = MockTransactions::new();
    transaction_handler
        .expect_is_repeated_transaction()
        .withf(move |sig| sig.eq(&signature))
        .returning(|_| {
            Err(RelayerServerError::StorageError(StorageError::DatabaseError(anyhow!(
                "database error"
            ))))
        });
    let options = MockOptions {
        chain_id: CHAIN_ID,
        providers: HashMap::new(),
        transaction_handler,
        account_handler: MockAccounts::new(),
        token_price: MockTokenPrice::new(),
        consumer: MockConsumers::new(),
        producer: MockProducers::new(),
    };
    let app = create_app(options).await.unwrap();

    let request = TestRequest::post().uri("/api/v2/transact").set_json(data).to_request();
    let response: ApiResponse<RelayTransactResponse> = call_and_read_body_json(&app, request).await;
    assert_eq!(response.code, ResponseCode::DatabaseError as i32);
    assert!(response.data.is_none());
    assert!(response.message.is_some());
}

#[actix_rt::test]
async fn test_with_chain_config_not_found() {
    let mut data = transact_request_data();
    data.chain_id = 1u64;
    let signature = data.signature.clone();
    let mut transaction_handler = MockTransactions::new();
    transaction_handler
        .expect_is_repeated_transaction()
        .withf(move |sig| sig.eq(&signature))
        .returning(|_| Ok(false));
    let options = MockOptions {
        chain_id: CHAIN_ID,
        providers: HashMap::new(),
        transaction_handler,
        account_handler: MockAccounts::new(),
        token_price: MockTokenPrice::new(),
        consumer: MockConsumers::new(),
        producer: MockProducers::new(),
    };
    let app = create_app(options).await.unwrap();

    let request = TestRequest::post().uri("/api/v2/transact").set_json(data).to_request();
    let response: ApiResponse<RelayTransactResponse> = call_and_read_body_json(&app, request).await;
    assert_eq!(response.code, ResponseCode::ChainIdNotFound as i32);
    assert!(response.data.is_none());
    assert!(response.message.is_some());
}

#[actix_rt::test]
async fn test_with_find_sender_error() {
    let mut data = transact_request_data();
    data.asset_symbol = "mUSD".to_string();
    let signature = data.signature.clone();
    let mut transaction_handler = MockTransactions::new();
    transaction_handler
        .expect_is_repeated_transaction()
        .withf(move |sig| sig.eq(&signature))
        .returning(|_| Ok(false));
    let producer = MockProducers::new();
    let options = MockOptions {
        chain_id: CHAIN_ID,
        providers: HashMap::new(),
        transaction_handler,
        account_handler: MockAccounts::new(),
        token_price: MockTokenPrice::new(),
        consumer: MockConsumers::new(),
        producer,
    };
    let app = create_app(options).await.unwrap();

    let request = TestRequest::post().uri("/api/v2/transact").set_json(data).to_request();
    let response: ApiResponse<RelayTransactResponse> = call_and_read_body_json(&app, request).await;
    assert_eq!(response.code, ResponseCode::UnsupportedTransaction as i32);
    assert!(response.data.is_none());
    assert!(response.message.is_some());
}

#[actix_rt::test]
async fn test_with_send_error() {
    let data = transact_request_data();
    let signature = data.signature.clone();
    let mut transaction_handler = MockTransactions::new();
    transaction_handler
        .expect_is_repeated_transaction()
        .withf(move |sig| sig.eq(&signature))
        .returning(|_| Ok(false));
    transaction_handler
        .expect_find_by_id()
        .withf(|id| id == "123456")
        .returning(|_| {
            let mut transaction = default_transaction();
            transaction.status = TransactStatus::Succeeded;
            Ok(Some(Document::new(
                "123456".to_string(),
                1234567890u64,
                1234567891u64,
                transaction,
            )))
        });
    let mut producer = MockProducers::new();
    producer
        .expect_send()
        .withf(|data| data.chain_id == CHAIN_ID)
        .returning(|_| Err(RelayerServerError::QueueSendError("mock error".to_string())));
    let options = MockOptions {
        chain_id: CHAIN_ID,
        providers: HashMap::new(),
        transaction_handler,
        account_handler: MockAccounts::new(),
        token_price: MockTokenPrice::new(),
        consumer: MockConsumers::new(),
        producer,
    };
    let app = create_app(options).await.unwrap();

    let request = TestRequest::post().uri("/api/v2/transact").set_json(data).to_request();
    let response: ApiResponse<RelayTransactResponse> = call_and_read_body_json(&app, request).await;
    assert_eq!(response.code, ResponseCode::TransactionChannelError as i32);
}

fn transact_request_data() -> TransactRequestData {
    TransactRequestData {
        contract_param: TransactRequest {
            proof: Default::default(),
            root_hash: Default::default(),
            serial_numbers: vec![U256::from_str_radix(
                "0x19aaddbfd3840e5d9363793cc8a91c8e223db9775095316e528fe335db42956d",
                16,
            )
            .unwrap()],
            sig_hashes: vec![U256::from_str_radix(
                "0x0e5a093c5390514adad7e5277500319e7cc35d7682a4fa2ac84f4b5332909a5f",
                16,
            )
            .unwrap()],
            sig_pk: [
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 122, 235, 117, 149, 215, 12, 61, 182, 5, 183, 46, 6, 199, 169, 27,
                55, 117, 42, 27, 83,
            ],
            public_amount: U256::from_str_radix(
                "0x00000000000000000000000000000000000000000000000003fba0faba898000",
                16,
            )
            .unwrap(),
            relayer_fee_amount: U256::from_str_radix(
                "0x000000000000000000000000000000000000000000000000000aa87bee538000",
                16,
            )
            .unwrap(),
            out_commitments: vec![U256::from_str_radix(
                "0x1da10644733ab072dc6ea8aa6087d797b5002aa53238b753132448ba981102c5",
                16,
            )
            .unwrap()],
            out_rollup_fees: vec![U256::from_str_radix(
                "0x000000000000000000000000000000000000000000000000002386f26fc10000",
                16,
            )
            .unwrap()],
            public_recipient: Default::default(),
            relayer_address: Default::default(),
            out_encrypted_notes: vec![Bytes::from_str(
                "0x013b356d8d7b70e3896a4673b9a2c58904394a7d50bc92a6325b8\
                bedf6ec69ae938edaa562b23b50a7c62400ee344e6cedbb22233d53020d25e33650be5449b9ccd\
                94ca38c8ac66942c2d292b23149ec48b87de118acfab3895123e6eac243acf7a7055dbae309261\
                99852844ef19e2e43b065b697ae7a1faba33430240d380aa088ea5d207757780f412c401c503d7\
                3e3394703b6427a277f583a4bf368063966c32c4b3b238ebe0d60c544693d69c127529194da3bf\
                e5726064b96f7580802fa591dffea922139cfe2eccb6220d322a3",
            )
            .unwrap()],
            random_auditing_public_key: Default::default(),
            encrypted_auditor_notes: vec![],
        },
        spend_type: SpendType::Withdraw,
        bridge_type: BridgeType::Loop,
        chain_id: CHAIN_ID,
        asset_symbol: "MTT".to_string(),
        asset_decimals: 18,
        pool_address: "0x4F416Acfd1153F9Af782056e68607227Af29D931".to_string(),
        circuit_type: CircuitType::Transaction1x0,
        signature: "0x800157ae47e94a156c42584190c33362b13ff94a7e8f5ef6ffd602c8d19ae\
        0684a4da6afd3c10bae9bd252dd20a9388d86c617bacb807a236a0285603e4086d61b"
            .to_string(),
    }
}
