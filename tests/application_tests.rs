mod common;

use crate::common::TESTNET_CONFIG_PATH;
use mystiko_relayer::application::{run_application, ApplicationOptions};
use std::time::Duration;

#[actix_rt::test]
async fn test_run_application() {
    let options = ApplicationOptions::builder()
        .server_config_path(Some(TESTNET_CONFIG_PATH))
        .array_queue_capacity(10)
        .build();
    let _result = tokio::time::timeout(Duration::from_secs(15), run_application(options)).await;
}
