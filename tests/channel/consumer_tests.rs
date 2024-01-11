use crate::channel::{create_default_sender_and_receiver, MockProvider, MockProviders};
use crate::common::{
    default_transact_request_data, default_transaction, default_transaction_receipt, MockTokenPrice, MockTxManager,
};
use crate::handler::MockTransactions;
use ethers_core::types::{TxHash, U256};
use log::LevelFilter;
use mystiko_ethers::{Provider, ProviderWrapper};
use mystiko_relayer::channel::consumer::handler::TransactionConsumer;
use mystiko_relayer::channel::consumer::ConsumerHandler;
use mystiko_relayer_types::TransactRequestData;
use mystiko_storage::Document;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::sync::mpsc::Receiver;
use tokio::sync::RwLock;
use typed_builder::TypedBuilder;

#[test]
fn test_consumer_execution_success() {
    let rt = Runtime::new().unwrap();
    let _ = env_logger::builder()
        .filter_module("mystiko_relayer", LevelFilter::Debug)
        .try_init();
    rt.block_on(async {
        let chain_id = 99;
        let transaction_id = "123456";

        // mock providers
        let provider = MockProvider::new();
        let mut providers = HashMap::new();
        providers.insert(chain_id, provider);
        // mock transaction handler
        let mut transaction_handler = MockTransactions::new();
        // mock token price
        let mut token_price = MockTokenPrice::new();
        // mock tx manager
        let mut tx_manager = MockTxManager::new();
        // mock receiver and sender
        let mock = create_default_sender_and_receiver();
        let sender = mock.sender;
        let receiver = mock.receiver;

        // mock response
        let tx_hash = TxHash::random();
        // gas price
        tx_manager.expect_gas_price().returning(|_| Ok(U256::from(1000000)));
        // estimate gas
        tx_manager
            .expect_estimate_gas()
            .returning(|_, _| Ok(U256::from(1000000)));
        // token price swap
        token_price
            .expect_swap()
            .returning(|_, _, _, _, _| Ok(U256::from(1100000000000u64)));
        // send
        tx_manager.expect_send().returning(move |_, _| Ok(tx_hash));
        // transaction handler update
        transaction_handler.expect_update_by_id().returning(|_, _| {
            Ok(Some(Document::new(
                String::from("123456"),
                1234567890u64,
                1234567891u64,
                default_transaction(),
            )))
        });
        // wait confirm
        tx_manager
            .expect_confirm()
            .withf(move |hash, _| hash == &tx_hash)
            .returning(move |_, _| Ok(default_transaction_receipt(tx_hash)));

        // create consumer
        let mut consumer = setup(MockOptions {
            chain_id,
            is_tx_eip1559: false,
            main_asset_symbol: "MTT".to_string(),
            main_asset_decimals: 16,
            receiver,
            providers,
            transaction_handler,
            token_price,
            tx_manager,
        });

        tokio::spawn(async move {
            consumer.consume().await;
        });

        // send request to consumer
        let result = sender
            .send((transaction_id.to_string(), default_transact_request_data(chain_id)))
            .await;
        assert!(result.is_ok());
    });
}

#[derive(Debug, TypedBuilder)]
struct MockOptions {
    chain_id: u64,
    is_tx_eip1559: bool,
    main_asset_symbol: String,
    main_asset_decimals: u32,
    receiver: Receiver<(String, TransactRequestData)>,
    providers: HashMap<u64, MockProvider>,
    transaction_handler: MockTransactions,
    token_price: MockTokenPrice,
    tx_manager: MockTxManager,
}

fn setup(options: MockOptions) -> TransactionConsumer {
    let mut providers = MockProviders::new();
    let mut raw_providers = options
        .providers
        .into_iter()
        .map(|(chain_id, provider)| {
            let provider = Arc::new(Provider::new(ProviderWrapper::new(Box::new(provider))));
            (chain_id, provider)
        })
        .collect::<HashMap<_, _>>();
    providers.expect_get_provider().returning(move |chain_id| {
        raw_providers
            .remove(&chain_id)
            .ok_or(anyhow::anyhow!("No provider for chain_id {}", chain_id))
    });
    TransactionConsumer {
        chain_id: options.chain_id,
        is_tx_eip1559: options.is_tx_eip1559,
        main_asset_symbol: options.main_asset_symbol,
        main_asset_decimals: options.main_asset_decimals,
        receiver: options.receiver,
        providers: Arc::new(Box::new(providers)),
        handler: Arc::new(Box::new(options.transaction_handler)),
        token_price: Arc::new(RwLock::new(Box::new(options.token_price))),
        tx_manager: Box::new(options.tx_manager),
    }
}
