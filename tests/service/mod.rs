use std::collections::HashMap;
use std::sync::Arc;
use ethers_providers::MockProvider;
use crate::common::SERVER_CONFIG_TESTNET;
use mystiko_relayer::configs::load_server_config;
use mystiko_relayer::context::{Context, create_config};
use mystiko_relayer::database::init_sqlite_database;
use crate::handler::MockTransactions;

mod handshake_tests;
mod v1;
mod v2;

struct MockOptions {
    providers: HashMap<u64, MockProvider>,
    transaction_handler: MockTransactions,
    account_handler: MockAccount,
}

async fn run_application(options: MockOptions) {
    let server_config = Arc::new(load_server_config(Some(SERVER_CONFIG_TESTNET)).unwrap());
    let database = init_sqlite_database(None).await.unwrap();
    let (relayer_config, mystiko_config) = create_config(server_config.clone()).await.unwrap();

    let context = Context {
        server_config,
        relayer_config,
        mystiko_config,
        providers: Arc::new(Box::new(())),
        transaction_handler: Arc::new(Box::new(())),
        account_handler: Arc::new(Box::new(())),
        token_price: Arc::new(()),
    }
}

