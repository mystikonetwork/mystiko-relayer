use crate::common::{MockProviders, MockTokenPrice, SERVER_CONFIG_TESTNET};
use crate::handler::{MockAccounts, MockTransactions};
use ethers_providers::MockProvider;
use mystiko_ethers::{Provider, ProviderWrapper};
use mystiko_relayer::configs::load_server_config;
use mystiko_relayer::context::{create_config, Context};
use mystiko_relayer::database::init_sqlite_database;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

mod handshake_tests;
mod v1;
mod v2;

struct MockOptions {
    providers: HashMap<u64, MockProvider>,
    transaction_handler: MockTransactions,
    account_handler: MockAccounts,
    token_price: MockTokenPrice,
}

async fn run_application(options: MockOptions) {
    let server_config = Arc::new(load_server_config(Some(SERVER_CONFIG_TESTNET)).unwrap());
    let database = init_sqlite_database(None).await.unwrap();
    let (relayer_config, mystiko_config) = create_config(server_config.clone()).await.unwrap();

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

    let context = Context {
        server_config,
        relayer_config,
        mystiko_config,
        providers: Arc::new(Box::new(providers)),
        transaction_handler: Arc::new(Box::new(options.transaction_handler)),
        account_handler: Arc::new(Box::new(options.account_handler)),
        token_price: Arc::new(RwLock::new(Box::new(options.token_price))),
    };
}
