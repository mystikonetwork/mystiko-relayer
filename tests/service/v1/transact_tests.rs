use crate::channel::{MockConsumers, MockProducers};
use crate::common::{default_transaction, MockTokenPrice};
use crate::handler::{MockAccounts, MockTransactions};
use crate::service::{create_app, MockOptions};
use actix_web::test::{call_and_read_body_json, TestRequest};
use anyhow::anyhow;
use ethers_core::types::U256;
use mystiko_relayer::error::RelayerServerError;
use mystiko_relayer::service::v1::request::{
    G1PointStruct, G2PointStruct, ProofStruct, TransactRequestV1, TransactionTypeV1,
};
use mystiko_relayer::service::v1::response::TransactResponse;
use mystiko_relayer_types::response::{ApiResponse, ResponseCode};
use mystiko_relayer_types::TransactStatus;
use mystiko_storage::{Document, StorageError};
use mystiko_types::{BridgeType, CircuitType};
use std::collections::HashMap;

const CHAIN_ID: u64 = 5;

#[actix_rt::test]
async fn test_success() {
    let data = transact_request_v1();
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

    let request = TestRequest::post().uri("/transact").set_json(data).to_request();
    let response: ApiResponse<TransactResponse> = call_and_read_body_json(&app, request).await;
    assert_eq!(response.code, ResponseCode::Successful as i32);
}

#[actix_rt::test]
async fn test_main_success() {
    let mut data = transact_request_v1();
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

    let request = TestRequest::post().uri("/transact").set_json(data).to_request();
    let response: ApiResponse<TransactResponse> = call_and_read_body_json(&app, request).await;
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
    let mut data = transact_request_v1();
    data.chain_id = 0u64;

    let request = TestRequest::post().uri("/transact").set_json(data).to_request();
    let response: ApiResponse<TransactResponse> = call_and_read_body_json(&app, request).await;
    assert_eq!(response.code, ResponseCode::ValidateError as i32);
    assert!(response.message.is_some());
    assert!(response.data.is_none());
}

#[actix_rt::test]
async fn test_with_repeated_transaction() {
    let data = transact_request_v1();
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

    let request = TestRequest::post().uri("/transact").set_json(data).to_request();
    let response: ApiResponse<TransactResponse> = call_and_read_body_json(&app, request).await;
    assert_eq!(response.code, ResponseCode::RepeatedTransaction as i32);
    assert!(response.data.is_none());
    assert!(response.message.is_some());
}

#[actix_rt::test]
async fn test_with_database_error() {
    let data = transact_request_v1();
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

    let request = TestRequest::post().uri("/transact").set_json(data).to_request();
    let response: ApiResponse<TransactResponse> = call_and_read_body_json(&app, request).await;
    assert_eq!(response.code, ResponseCode::DatabaseError as i32);
    assert!(response.data.is_none());
    assert!(response.message.is_some());
}

#[actix_rt::test]
async fn test_with_chain_config_not_found() {
    let mut data = transact_request_v1();
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

    let request = TestRequest::post().uri("/transact").set_json(data).to_request();
    let response: ApiResponse<TransactResponse> = call_and_read_body_json(&app, request).await;
    assert_eq!(response.code, ResponseCode::ChainIdNotFound as i32);
    assert!(response.data.is_none());
    assert!(response.message.is_some());
}

#[actix_rt::test]
async fn test_with_pool_contract_not_found() {
    let mut data = transact_request_v1();
    data.pool_address = "0x4F416Acfd1153F9Af782056e68607227Af29D932".to_string();
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

    let request = TestRequest::post().uri("/transact").set_json(data).to_request();
    let response: ApiResponse<TransactResponse> = call_and_read_body_json(&app, request).await;
    assert_eq!(response.code, ResponseCode::Failed as i32);
    assert!(response.data.is_none());
    assert!(response.message.is_some());
}

#[actix_rt::test]
async fn test_with_find_sender_error() {
    let mut data = transact_request_v1();
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

    let request = TestRequest::post().uri("/transact").set_json(data).to_request();
    let response: ApiResponse<TransactResponse> = call_and_read_body_json(&app, request).await;
    assert_eq!(response.code, ResponseCode::UnsupportedTransaction as i32);
    assert!(response.data.is_none());
    assert!(response.message.is_some());
}

#[actix_rt::test]
async fn test_parse_transact_request_error() {
    let mut data = transact_request_v1();
    data.random_auditing_public_key = "0x000".to_string();
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

    let request = TestRequest::post().uri("/transact").set_json(data).to_request();
    let response: ApiResponse<TransactResponse> = call_and_read_body_json(&app, request).await;
    assert_eq!(response.code, ResponseCode::Failed as i32);
}

#[actix_rt::test]
async fn test_with_find_by_transaction_id_error() {
    let mut data = transact_request_v1();
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
        .returning(|_| Err(RelayerServerError::AnyhowError(anyhow!("anyhow error"))));
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

    let request = TestRequest::post().uri("/transact").set_json(data).to_request();
    let response: ApiResponse<TransactResponse> = call_and_read_body_json(&app, request).await;
    assert_eq!(response.code, ResponseCode::TransactionNotFound as i32);
}

#[actix_rt::test]
async fn test_with_send_error() {
    let data = transact_request_v1();
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

    let request = TestRequest::post().uri("/transact").set_json(data).to_request();
    let response: ApiResponse<TransactResponse> = call_and_read_body_json(&app, request).await;
    assert_eq!(response.code, ResponseCode::TransactionChannelError as i32);
}

#[actix_rt::test]
async fn test_with_transaction_status_error() {
    let data = transact_request_v1();
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
            transaction.status = TransactStatus::Failed;
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

    let request = TestRequest::post().uri("/transact").set_json(data).to_request();
    let response: ApiResponse<TransactResponse> = call_and_read_body_json(&app, request).await;
    assert_eq!(response.code, ResponseCode::Failed as i32);
    assert!(response.data.is_none());
    assert!(response.message.is_some());
}

fn transact_request_v1() -> TransactRequestV1 {
    TransactRequestV1 {
        proof: ProofStruct {
            a: G1PointStruct {
                x: Default::default(),
                y: Default::default(),
            },
            b: G2PointStruct {
                x: [U256::zero(), U256::zero()],
                y: [U256::zero(), U256::zero()],
            },
            c: G1PointStruct {
                x: Default::default(),
                y: Default::default(),
            },
        },
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
        sig_pk: "0x0000000000000000000000007e47ad819977cf3a513a544ed977791ceeb9688a".to_string(),
        public_amount: U256::from_str_radix("0x00000000000000000000000000000000000000000000000003fba0faba898000", 16)
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
        out_encrypted_notes: vec![
            "4272275035674925470534869677870077469352725316683400840467655180589816040683".to_string(),
        ],
        random_auditing_public_key: "5467781221150212220743129070059817005710506435433685712606005795860949029646"
            .to_string(),
        encrypted_auditor_notes: vec![
            "4272275035674925470534869677870077469352725316683400840467655180589816040683".to_string(),
            "20452133727401060957272588420048718934339143694633738390375682117144709087485".to_string(),
        ],
        signature: "0x800157ae47e94a156c42584190c33362b13ff94a7e8f5ef6ffd602c8d19ae\
        0684a4da6afd3c10bae9bd252dd20a9388d86c617bacb807a236a0285603e4086d61b"
            .to_string(),
        transaction_type: TransactionTypeV1::Withdraw,
        chain_id: 5,
        pool_address: "0x4F416Acfd1153F9Af782056e68607227Af29D931".to_string(),
        asset_symbol: "MTT".to_string(),
        bridge_type: BridgeType::Loop,
        circuit_type: CircuitType::Transaction1x0,
        relayer_fee_amount: U256::from_str_radix(
            "0x0000000000000000000000000000000000000000000000007ce66c50e2840000",
            16,
        )
        .unwrap(),
        relayer_address: Default::default(),
    }
}
