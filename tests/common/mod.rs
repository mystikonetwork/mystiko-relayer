use mystiko_relayer::configs::load_server_config;
use mystiko_relayer::configs::server::ServerConfig;
use mystiko_relayer::context::Context;
use mystiko_relayer::database::Database;
use mystiko_storage::SqlStatementFormatter;
use mystiko_storage_sqlite::SqliteStorage;
use std::sync::Arc;

#[allow(dead_code)]
const SERVER_CONFIG_TESTNET: &str = "tests/files/configs/config_testnet.toml";
#[allow(dead_code)]
const SERVER_CONFIG_MAINNET: &str = "tests/files/configs/config_mainnet.toml";
#[allow(dead_code)]
pub const RELAYER_CONFIG_PATH: &str = "tests/files/relayer_config.json";
#[allow(dead_code)]
pub const SERVER_CONFIG_INVALID_ID: &str = "tests/files/configs/config_invalid_id.toml";
#[allow(dead_code)]
pub const SERVER_CONFIG_INVALID_SYMBOL: &str = "tests/files/configs/config_invalid_symbol.toml";
#[allow(dead_code)]
pub const SERVER_CONFIG_INVALID_VERSION: &str = "tests/files/configs/config_invalid_version.toml";

#[allow(unused)]
pub async fn create_default_server_config(testnet: bool) -> ServerConfig {
    if testnet {
        load_server_config(Some(SERVER_CONFIG_TESTNET)).unwrap()
    } else {
        load_server_config(Some(SERVER_CONFIG_MAINNET)).unwrap()
    }
}

#[allow(unused)]
pub async fn create_default_database_in_memory() -> Database<SqlStatementFormatter, SqliteStorage> {
    let storage = SqliteStorage::from_memory().await.unwrap();
    let database = Database::new(SqlStatementFormatter::sqlite(), storage);
    database.migrate().await.unwrap();
    database
}

#[allow(unused)]
pub async fn create_default_context() -> Context {
    let server_config = create_default_server_config(true).await;
    let database = create_default_database_in_memory().await;
    Context::new(Arc::new(server_config), database).await.unwrap()
}
