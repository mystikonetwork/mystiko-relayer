mod consumer_tests;
mod producer_tests;

use crate::common::{TestServer, SERVER_CONFIG_ID_NOT_FOUND, TEST_RELAYER_CONFIG_SINGLE_PATH};
use mystiko_relayer::channel::transact_channel;
use mystiko_relayer::configs::load_server_config;
use mystiko_relayer_config::wrapper::relayer::RelayerConfig;
use mystiko_types::AssetType;

#[actix_rt::test]
async fn init_provider_not_found() {
    let server = TestServer::new(None).await.unwrap();
    let app_state = server.app_state;
    let server_config = load_server_config(Some(SERVER_CONFIG_ID_NOT_FOUND)).unwrap();
    let result = transact_channel::init(
        &server_config,
        &app_state.relayer_config,
        server.providers.clone(),
        server.transaction_handler.clone(),
        server.token_price.clone(),
        10,
    )
    .await;
    assert!(result.is_err());
}

#[actix_rt::test]
async fn find_producer_by_id_and_symbol_successful() {
    let server = TestServer::new(None).await.unwrap();
    let result = transact_channel::find_producer_by_id_and_symbol(&server.senders, 5, "Mtt", AssetType::Erc20);
    assert!(result.is_some());
}

#[actix_rt::test]
async fn find_producer_by_id_and_symbol_none() {
    let server = TestServer::new(None).await.unwrap();
    let result = transact_channel::find_producer_by_id_and_symbol(&server.senders, 5, "mUSD", AssetType::Erc20);
    assert!(result.is_none());
}

#[actix_rt::test]
#[should_panic(expected = "chain id 97 config not found in relayer config")]
async fn init_chain_id_not_found() {
    let server = TestServer::new(None).await.unwrap();
    let app_state = server.app_state;
    let relayer_config = RelayerConfig::from_json_file(TEST_RELAYER_CONFIG_SINGLE_PATH)
        .await
        .unwrap();
    let _ = transact_channel::init(
        &app_state.server_config,
        &relayer_config,
        server.providers.clone(),
        server.transaction_handler.clone(),
        server.token_price.clone(),
        10,
    )
    .await;
}
