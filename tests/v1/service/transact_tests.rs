use crate::common::{TestServer, TOKEN_PRICE_CONFIG_PATH};
use actix_rt::Runtime;
use actix_web::test::{call_and_read_body_json, init_service, TestRequest};
use actix_web::web::Data;
use actix_web::App;
use ethereum_types::U64;
use ethers_core::abi::AbiEncode;
use ethers_core::types::{Transaction, TransactionReceipt, TxHash, U256};
use ethers_providers::MockProvider;
use mockito::Matcher;
use mystiko_fs::read_file_bytes;
use mystiko_relayer::channel::transact_channel;
use mystiko_relayer::error::ResponseError;
use mystiko_relayer::v1::request::{G1PointStruct, G2PointStruct, ProofStruct, TransactRequestV1, TransactionTypeV1};
use mystiko_relayer::v1::response::TransactResponse;
use mystiko_relayer::v1::service::{parse_transact_request, transact_v1};
use mystiko_relayer_types::response::{ApiResponse, ResponseCode};
use mystiko_server_utils::token_price::query::CurrencyQuoteResponse;
use mystiko_types::{BridgeType, CircuitType};

fn valid_transact_request_data_v1() -> TransactRequestV1 {
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
        chain_id: 97,
        pool_address: "0x4F416Acfd1153F9Af782056e68607227Af29D931".to_string(),
        asset_symbol: "BNB".to_string(),
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

#[test]
fn test_send_successful_main_v1() {
    let rt = Runtime::new().unwrap();

    rt.block_on(async {
        let mock_provider = MockProvider::new();
        let mut server = TestServer::new(Some(mock_provider.clone())).await.unwrap();

        // init service
        let app = init_service(
            App::new()
                .app_data(Data::new(server.app_state.clone()))
                .app_data(Data::new(server.senders.clone()))
                .app_data(Data::new(server.transaction_handler.clone()))
                .service(transact_v1),
        )
        .await;

        // mock consumer
        let gas = U256::from(100_000_000_000u64);
        let nonce = U256::from(100);
        let tx_hash = TxHash::random();
        let block_number = U64::from(10000);

        // push execution transaction response
        let transaction_receipt = TransactionReceipt {
            transaction_hash: tx_hash,
            transaction_index: Default::default(),
            block_hash: None,
            block_number: None,
            from: Default::default(),
            to: None,
            cumulative_gas_used: Default::default(),
            gas_used: None,
            contract_address: None,
            logs: vec![],
            status: Some(U64::from(1)),
            root: None,
            logs_bloom: Default::default(),
            transaction_type: None,
            effective_gas_price: None,
            other: Default::default(),
        };
        let transaction_info = Transaction {
            hash: tx_hash,
            nonce,
            block_hash: None,
            block_number: Some(U64::from(9000)),
            transaction_index: None,
            from: Default::default(),
            to: None,
            value: Default::default(),
            gas_price: None,
            gas,
            input: Default::default(),
            v: Default::default(),
            r: Default::default(),
            s: Default::default(),
            transaction_type: None,
            access_list: None,
            max_priority_fee_per_gas: None,
            max_fee_per_gas: None,
            chain_id: None,
            other: Default::default(),
        };
        // get_transaction_receipt
        mock_provider.push(Some(transaction_receipt)).unwrap();
        // set block number
        mock_provider.push(block_number).unwrap();
        // get transaction
        mock_provider.push(Some(transaction_info)).unwrap();
        mock_provider.push(tx_hash).unwrap();
        mock_provider.push(nonce).unwrap();

        // mock server
        let id_bytes = read_file_bytes(TOKEN_PRICE_CONFIG_PATH).await.unwrap();
        let currency_quote: CurrencyQuoteResponse = serde_json::from_slice(&id_bytes).unwrap();
        let mock = server
            .mock_server
            .mock("GET", "/v2/cryptocurrency/quotes/latest")
            .match_query(Matcher::Any)
            .with_body(serde_json::to_string(&currency_quote).expect("Failed to serialize struct to JSON"))
            .create_async()
            .await;

        // run consumers
        for consumer in server.consumers {
            rt.spawn(async {
                consumer.run().await;
            });
        }

        let gas = U256::from(100_000_000_000u64);
        let nonce = U256::from(100);
        let price = U256::from(1000000);
        mock_provider.push(price).unwrap();
        mock_provider.push(gas).unwrap();
        mock_provider.push(nonce).unwrap();
        mock_provider.push(price).unwrap();

        let req = TestRequest::post()
            .uri("/transact")
            .set_json(valid_transact_request_data_v1())
            .to_request();
        let resp: ApiResponse<TransactResponse> = call_and_read_body_json(&app, req).await;
        assert_eq!(resp.code, ResponseCode::Successful as i32);
        assert!(resp.message.is_none());
        assert!(resp.data.is_some());
        assert_eq!(resp.data.unwrap().hash, tx_hash.encode_hex());
        mock.assert_async().await;
    });
}

#[test]
fn test_send_successful_erc20_v1() {
    let rt = Runtime::new().unwrap();

    rt.block_on(async {
        let mock_provider = MockProvider::new();
        let mut server = TestServer::new(Some(mock_provider.clone())).await.unwrap();

        // init service
        let app = init_service(
            App::new()
                .app_data(Data::new(server.app_state.clone()))
                .app_data(Data::new(server.senders.clone()))
                .app_data(Data::new(server.transaction_handler.clone()))
                .service(transact_v1),
        )
        .await;

        // mock consumer
        let gas = U256::from(100_000_000_000u64);
        let nonce = U256::from(100);
        let tx_hash = TxHash::random();
        let block_number = U64::from(10000);

        // push execution transaction response
        let transaction_receipt = TransactionReceipt {
            transaction_hash: tx_hash,
            transaction_index: Default::default(),
            block_hash: None,
            block_number: None,
            from: Default::default(),
            to: None,
            cumulative_gas_used: Default::default(),
            gas_used: None,
            contract_address: None,
            logs: vec![],
            status: Some(U64::from(1)),
            root: None,
            logs_bloom: Default::default(),
            transaction_type: None,
            effective_gas_price: None,
            other: Default::default(),
        };
        let transaction_info = Transaction {
            hash: tx_hash,
            nonce,
            block_hash: None,
            block_number: Some(U64::from(9000)),
            transaction_index: None,
            from: Default::default(),
            to: None,
            value: Default::default(),
            gas_price: None,
            gas,
            input: Default::default(),
            v: Default::default(),
            r: Default::default(),
            s: Default::default(),
            transaction_type: None,
            access_list: None,
            max_priority_fee_per_gas: None,
            max_fee_per_gas: None,
            chain_id: None,
            other: Default::default(),
        };
        // get_transaction_receipt
        mock_provider.push(Some(transaction_receipt)).unwrap();
        // set block number
        mock_provider.push(block_number).unwrap();
        // get transaction
        mock_provider.push(Some(transaction_info)).unwrap();
        mock_provider.push(tx_hash).unwrap();
        mock_provider.push(nonce).unwrap();

        // mock server
        let id_bytes = read_file_bytes(TOKEN_PRICE_CONFIG_PATH).await.unwrap();
        let currency_quote: CurrencyQuoteResponse = serde_json::from_slice(&id_bytes).unwrap();
        let mock = server
            .mock_server
            .mock("GET", "/v2/cryptocurrency/quotes/latest")
            .match_query(Matcher::Any)
            .with_body(serde_json::to_string(&currency_quote).expect("Failed to serialize struct to JSON"))
            .create_async()
            .await;

        // run consumers
        for consumer in server.consumers {
            rt.spawn(async {
                consumer.run().await;
            });
        }

        let gas = U256::from(100_000_000_000u64);
        let nonce = U256::from(100);
        let price = U256::from(1000000);
        mock_provider.push(price).unwrap();
        mock_provider.push(gas).unwrap();
        mock_provider.push(nonce).unwrap();
        mock_provider.push(price).unwrap();

        let mut transact_data = valid_transact_request_data_v1();
        transact_data.asset_symbol = "mUSD".to_string();

        let req = TestRequest::post()
            .uri("/transact")
            .set_json(transact_data)
            .to_request();
        let resp: ApiResponse<TransactResponse> = call_and_read_body_json(&app, req).await;
        assert_eq!(resp.code, ResponseCode::Successful as i32);
        assert!(resp.message.is_none());
        assert!(resp.data.is_some());
        assert_eq!(resp.data.unwrap().hash, tx_hash.encode_hex());
        mock.assert_async().await;
    });
}

#[actix_rt::test]
async fn test_invalid_request_v1() {
    let server = TestServer::new(None).await.unwrap();

    let app = init_service(
        App::new()
            .app_data(Data::new(server.app_state.clone()))
            .app_data(Data::new(server.senders.clone()))
            .app_data(Data::new(server.transaction_handler.clone()))
            .service(transact_v1),
    )
    .await;

    let mut transact_data = valid_transact_request_data_v1();
    transact_data.circuit_type = CircuitType::Rollup1;

    let req = TestRequest::post()
        .uri("/transact")
        .set_json(transact_data)
        .to_request();
    let resp: ApiResponse<TransactResponse> = call_and_read_body_json(&app, req).await;
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
async fn test_repeated_transaction_v1() {
    let server = TestServer::new(None).await.unwrap();
    let app = init_service(
        App::new()
            .app_data(Data::new(server.app_state.clone()))
            .app_data(Data::new(server.senders.clone()))
            .app_data(Data::new(server.transaction_handler.clone()))
            .service(transact_v1),
    )
    .await;

    let request = valid_transact_request_data_v1();
    let result = server
        .transaction_handler
        .create_by_request(parse_transact_request(request, 16).unwrap())
        .await;
    assert!(result.is_ok());

    let req = TestRequest::post()
        .uri("/transact")
        .set_json(valid_transact_request_data_v1())
        .to_request();
    let resp: ApiResponse<TransactResponse> = call_and_read_body_json(&app, req).await;
    assert_eq!(resp.code, ResponseCode::RepeatedTransaction as i32);
    assert!(resp.message.is_some());
    assert_eq!(resp.message.unwrap(), ResponseError::RepeatedTransaction.to_string());
}

#[actix_rt::test]
async fn test_chain_id_not_found_v1() {
    let server = TestServer::new(None).await.unwrap();

    let app = init_service(
        App::new()
            .app_data(Data::new(server.app_state.clone()))
            .app_data(Data::new(server.senders.clone()))
            .app_data(Data::new(server.transaction_handler.clone()))
            .service(transact_v1),
    )
    .await;

    let mut transact_data = valid_transact_request_data_v1();
    transact_data.chain_id = 99999;

    let req = TestRequest::post()
        .uri("/transact")
        .set_json(transact_data)
        .to_request();
    let resp: ApiResponse<TransactResponse> = call_and_read_body_json(&app, req).await;
    assert_eq!(resp.code, ResponseCode::ChainIdNotFound as i32);
    assert!(resp.message.is_some());
    assert_eq!(
        resp.message.unwrap(),
        ResponseError::ChainIdNotFoundInRelayerConfig { chain_id: 99999 }.to_string()
    );
}

#[actix_rt::test]
async fn test_send_transaction_to_queue_failed_v1() {
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

    let app = init_service(
        App::new()
            .app_data(Data::new(server.app_state.clone()))
            .app_data(Data::new(server.senders.clone()))
            .app_data(Data::new(server.transaction_handler.clone()))
            .service(transact_v1),
    )
    .await;

    drop(server.senders);

    let req = TestRequest::post()
        .uri("/transact")
        .set_json(valid_transact_request_data_v1())
        .to_request();
    let resp: ApiResponse<TransactResponse> = call_and_read_body_json(&app, req).await;
    assert_eq!(resp.code, ResponseCode::TransactionChannelError as i32);
}

#[actix_rt::test]
async fn test_unsupported_transaction_v1() {
    // create test server
    let mock_provider = MockProvider::new();
    let server = TestServer::new(Some(mock_provider.clone())).await.unwrap();

    let app = init_service(
        App::new()
            .app_data(Data::new(server.app_state.clone()))
            .app_data(Data::new(server.senders.clone()))
            .app_data(Data::new(server.transaction_handler.clone()))
            .service(transact_v1),
    )
    .await;

    let gas = U256::from(100_000_000_000u64);
    let nonce = U256::from(100);
    let price = U256::from(1000000);
    mock_provider.push(price).unwrap();
    mock_provider.push(gas).unwrap();
    mock_provider.push(nonce).unwrap();
    mock_provider.push(price).unwrap();

    let mut transact_data = valid_transact_request_data_v1();
    transact_data.asset_symbol = "mtt1".to_string();

    let req = TestRequest::post()
        .uri("/transact")
        .set_json(transact_data)
        .to_request();
    let resp: ApiResponse<TransactResponse> = call_and_read_body_json(&app, req).await;
    assert_eq!(resp.code, ResponseCode::UnsupportedTransaction as i32);
}
