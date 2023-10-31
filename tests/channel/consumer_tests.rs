use crate::common::{get_valid_transact_request_data, TestServer, TOKEN_PRICE_CONFIG_PATH};
use ethereum_types::{U256, U64};
use ethers_core::types::{Transaction, TransactionReceipt, TxHash};
use ethers_providers::MockProvider;
use log::error;
use mockito::Matcher;
use mystiko_fs::read_file_bytes;
use mystiko_relayer::channel::transact_channel;
use mystiko_relayer::database::Database;
use mystiko_relayer::handler::transaction::TransactionHandler;
use mystiko_relayer_types::TransactStatus;
use mystiko_server_utils::token_price::CurrencyQuoteResponse;
use mystiko_storage::SqlStatementFormatter;
use mystiko_storage_sqlite::SqliteStorage;
use mystiko_types::AssetType;
use serial_test::file_serial;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;
use tokio::runtime::Runtime;

#[test]
fn test_consumer_execution_successful() {
    // tokio runtime
    let rt = Runtime::new().unwrap();

    rt.block_on(async {
        let mock_provider = MockProvider::new();
        let mut server = TestServer::new(Some(mock_provider.clone())).await.unwrap();
        assert!(!server.consumers.is_empty());

        // mock provider
        let gas = U256::from(100_000_000_000u64);
        let nonce = U256::from(100);
        let price = U256::from(1000000);
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
        mock_provider.push(price).unwrap();
        mock_provider.push(price).unwrap();
        mock_provider.push(gas).unwrap();
        mock_provider.push(nonce).unwrap();
        mock_provider.push(price).unwrap();

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

        // sender send a msg
        let sender = transact_channel::find_producer_by_id_and_symbol(&server.senders, 5, "eth", AssetType::Main);
        assert!(sender.is_some());
        let sender = sender.unwrap();
        let res = sender.send(get_valid_transact_request_data()).await;
        assert!(res.is_ok());

        // run consumers
        for consumer in server.consumers {
            rt.spawn(async {
                consumer.run().await;
            });
        }

        // find transaction
        let uuid = res.unwrap().id;

        // wait transaction successful
        loop {
            let transaction = server.transaction_handler.find_by_id(&uuid).await;
            assert!(transaction.is_ok());
            let transaction = transaction.unwrap();
            assert!(transaction.is_some());
            let transaction = transaction.unwrap();
            assert_eq!(transaction.id, uuid);
            if transaction.data.status == TransactStatus::Failed {
                error!("transaction failed: {:?}", transaction.data.error_message);
            }
            assert_ne!(transaction.data.status, TransactStatus::Failed);
            if transaction.data.status == TransactStatus::Succeeded {
                break;
            }
            sleep(Duration::from_secs(3));
        }
        mock.assert_async().await;
    });
}

#[test]
fn test_consumer_execution_failed() {
    let rt = Runtime::new().unwrap();

    rt.block_on(async {
        let mock_provider = MockProvider::new();
        let server = TestServer::new(Some(mock_provider.clone())).await.unwrap();
        assert!(!server.consumers.is_empty());

        // sender send a msg
        let sender = transact_channel::find_producer_by_id_and_symbol(&server.senders, 5, "eth", AssetType::Main);
        assert!(sender.is_some());
        let sender = sender.unwrap();
        let res = sender.send(get_valid_transact_request_data()).await;
        assert!(res.is_ok());

        // run consumers
        for consumer in server.consumers {
            rt.spawn(async {
                consumer.run().await;
            });
        }

        // find transaction
        let uuid = res.unwrap().id;

        // wait transaction failed
        loop {
            let transaction = server.transaction_handler.find_by_id(&uuid).await;
            assert!(transaction.is_ok());
            let transaction = transaction.unwrap();
            assert!(transaction.is_some());
            let transaction = transaction.unwrap();
            assert_eq!(transaction.id, uuid);
            assert_ne!(transaction.data.status, TransactStatus::Succeeded);
            if transaction.data.status == TransactStatus::Failed {
                break;
            }
            sleep(Duration::from_secs(3));
        }
    });
}

#[test]
#[file_serial]
fn test_validate_relayer_fee_error() {
    // tokio runtime
    let rt = Runtime::new().unwrap();

    rt.block_on(async {
        let mock_provider = MockProvider::new();
        let mut server = TestServer::new(Some(mock_provider.clone())).await.unwrap();
        assert!(!server.consumers.is_empty());

        // mock provider
        let gas = U256::from(999_000_000_000_000_000u64);
        let nonce = U256::from(100);
        let price = U256::from(1000000);
        let tx_hash = TxHash::random();

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
            block_number: None,
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
        // get transaction
        mock_provider.push(Some(transaction_info)).unwrap();
        mock_provider.push(tx_hash).unwrap();
        mock_provider.push(price).unwrap();
        mock_provider.push(price).unwrap();
        mock_provider.push(price).unwrap();
        mock_provider.push(gas).unwrap();
        mock_provider.push(nonce).unwrap();
        mock_provider.push(price).unwrap();

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

        // sender send a msg
        let sender = transact_channel::find_producer_by_id_and_symbol(&server.senders, 5, "eth", AssetType::Main);
        assert!(sender.is_some());
        let sender = sender.unwrap();
        let res = sender.send(get_valid_transact_request_data()).await;
        assert!(res.is_ok());

        // run consumers
        for consumer in server.consumers {
            rt.spawn(async {
                consumer.run().await;
            });
        }

        // find transaction
        let uuid = res.unwrap().id;

        // wait transaction failed
        loop {
            let transaction = server.transaction_handler.find_by_id(&uuid).await;
            assert!(transaction.is_ok());
            let transaction = transaction.unwrap();
            assert!(transaction.is_some());
            let transaction = transaction.unwrap();
            assert_eq!(transaction.id, uuid);
            assert_ne!(transaction.data.status, TransactStatus::Succeeded);
            if transaction.data.status == TransactStatus::Failed {
                break;
            }
            sleep(Duration::from_secs(3));
        }

        mock.assert_async().await;
    });
}

#[test]
#[file_serial]
fn test_max_retry_update_transaction_status() {
    // tokio runtime
    let rt = Runtime::new().unwrap();

    rt.block_on(async {
        let mock_provider = MockProvider::new();
        let server = TestServer::new(Some(mock_provider.clone())).await.unwrap();
        assert!(!server.consumers.is_empty());

        // sender send a msg
        let sender = transact_channel::find_producer_by_id_and_symbol(&server.senders, 5, "eth", AssetType::Main);
        assert!(sender.is_some());
        let sender = sender.unwrap();
        let res = sender.send(get_valid_transact_request_data()).await;
        assert!(res.is_ok());

        // run consumers
        for mut consumer in server.consumers {
            consumer.handler = Arc::new(TransactionHandler::new(Arc::new(Database::new(
                SqlStatementFormatter::sqlite(),
                SqliteStorage::from_memory().await.unwrap(),
            ))));
            rt.spawn(async {
                consumer.run().await;
            });
        }

        sleep(Duration::from_secs(15));
    });
}
