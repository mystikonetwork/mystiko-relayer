mod common;

use crate::common::TESTNET_CONFIG_PATH;
use mystiko_relayer::application::{run_application, ApplicationOptions};
use std::fs::remove_file;
use std::time::Duration;

#[actix_rt::test]
async fn test_run_application() {
    let options = ApplicationOptions::builder()
        .server_config_path(TESTNET_CONFIG_PATH)
        .array_queue_capacity(10)
        .build();
    let _result = tokio::time::timeout(Duration::from_secs(15), run_application(options)).await;
    // delete db.sqlite
    remove_file("./tests/files/db.sqlite").unwrap();
}
