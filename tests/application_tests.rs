use crate::common::SERVER_CONFIG_TESTNET;
use mystiko_relayer::application::{run_application, ApplicationOptions};
use mystiko_relayer::configs::load_server_config;
use mystiko_storage::SqlStatementFormatter;
use mystiko_storage_sqlite::SqliteStorage;
use std::sync::Arc;

mod common;

#[actix_rt::test]
async fn test_run_application() {
    let server_config = Arc::new(load_server_config(Some(SERVER_CONFIG_TESTNET)).unwrap());
    let options = ApplicationOptions::<SqlStatementFormatter, SqliteStorage>::from_server_config(server_config)
        .await
        .unwrap();
    let _ = tokio::time::timeout(std::time::Duration::from_secs(5), run_application(options)).await;
}
