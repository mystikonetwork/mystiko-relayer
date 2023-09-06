use anyhow::Result;
use mystiko_relayer::application::{run_application, ApplicationOptions};

pub const DEFAULT_SERVER_CONFIG_PATH: &str = "./config.toml";
pub const ARRAY_QUEUE_CAPACITY: usize = 50;

#[actix_web::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let server_config_path = args
        .get(1)
        .map(|path| path.as_str())
        .unwrap_or(DEFAULT_SERVER_CONFIG_PATH);

    run_application(
        ApplicationOptions::builder()
            .server_config_path(server_config_path)
            .array_queue_capacity(ARRAY_QUEUE_CAPACITY)
            .build(),
    )
    .await
}
