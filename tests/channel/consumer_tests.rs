use crate::channel::{create_default_channel, MockProvider, MockProviders};
use crate::common::{MockTokenPrice, MockTxManager};
use crate::handler::MockTransactions;
use actix_web::web::Data;
use mystiko_ethers::{Provider, ProviderWrapper};
use mystiko_relayer::channel::consumer::handler::TransactionConsumer;
use mystiko_relayer::service::find_sender;
use mystiko_relayer_types::TransactRequestData;
use mystiko_types::AssetType;
use serial_test::file_serial;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::sync::mpsc::Receiver;
use tokio::sync::RwLock;
use typed_builder::TypedBuilder;

#[test]
fn test_consumer_execution_success() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        // mock
        let chain_id = 99;

        // mock providers
        let provider = MockProvider::new();
        let mut providers = HashMap::new();
        providers.insert(chain_id, provider);

        // mock transaction handler
        let transaction_handler = MockTransactions::new();

        // mock token price
        let token_price = MockTokenPrice::new();

        // mock tx manager
        let tx_manager = MockTxManager::new();

        // create consumer
        let consumer = setup(MockOptions {
            chain_id,
            is_tx_eip1559: false,
            main_asset_symbol: "MTT".to_string(),
            main_asset_decimals: 16,
            receiver: (),
            providers,
            transaction_handler,
            token_price,
            tx_manager,
        });
    });
}

#[test]
fn test_consumer_execution_failed() {}

#[test]
#[file_serial]
fn test_validate_relayer_fee_error() {}

#[test]
#[file_serial]
fn test_max_retry_update_transaction_status() {}

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
