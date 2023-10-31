use crate::common::{
    TestServer, SERVER_CONFIG_ID_NOT_FOUND, SERVER_CONFIG_SYMBOL_INVALID, SERVER_CONFIG_VERSION_INVALID,
    TEST_RELAYER_CONFIG_PATH,
};
use mystiko_relayer::configs::load_server_config;
use mystiko_relayer_config::wrapper::relayer::RelayerConfig;

mod common;

#[actix_rt::test]
async fn test_find_account_successful() {
    let server = TestServer::new(None).await.unwrap();
    let server_config = server.app_state.server_config;
    let accounts = server_config.find_accounts(5);
    assert!(accounts.is_some());
}

#[actix_rt::test]
async fn test_find_accounts_available() {
    let server = TestServer::new(None).await.unwrap();
    let server_config = server.app_state.server_config;
    let accounts = server_config.find_accounts_available(5);
    assert!(accounts.is_some());
}

#[actix_rt::test]
async fn test_find_account_none() {
    let server = TestServer::new(None).await.unwrap();
    let server_config = server.app_state.server_config;
    let accounts = server_config.find_accounts(11111);
    assert!(accounts.is_none());
}

#[actix_rt::test]
async fn test_invalid_0() {
    let server_config = load_server_config(SERVER_CONFIG_ID_NOT_FOUND);
    assert!(server_config.is_ok());
    let relayer_config = RelayerConfig::from_json_file(TEST_RELAYER_CONFIG_PATH).await;
    assert!(relayer_config.is_ok());
    let server_config = server_config.unwrap();
    let relayer_config = relayer_config.unwrap();
    let validate = server_config.validation(&relayer_config);
    assert!(validate.is_err());
    assert_eq!(
        validate.unwrap_err().to_string().as_str(),
        "chain id 51111 not found in relayer config"
    );
}

#[actix_rt::test]
async fn test_invalid_1() {
    let server_config = load_server_config(SERVER_CONFIG_SYMBOL_INVALID);
    assert!(server_config.is_ok());
    let relayer_config = RelayerConfig::from_json_file(TEST_RELAYER_CONFIG_PATH).await;
    assert!(relayer_config.is_ok());
    let server_config = server_config.unwrap();
    let relayer_config = relayer_config.unwrap();
    let validate = server_config.validation(&relayer_config);
    assert!(validate.is_err());
    assert_eq!(
        validate.unwrap_err().to_string().as_str(),
        "chain_id 5 token TEST not found in relayer chain config"
    );
}

#[actix_rt::test]
async fn test_invalid_2() {
    let server_config = load_server_config(SERVER_CONFIG_VERSION_INVALID);
    assert!(server_config.is_ok());
    let relayer_config = RelayerConfig::from_json_file(TEST_RELAYER_CONFIG_PATH).await;
    assert!(relayer_config.is_ok());
    let server_config = server_config.unwrap();
    let relayer_config = relayer_config.unwrap();
    let validate = server_config.validation(&relayer_config);
    assert!(validate.is_err());
}
