use anyhow::Result;
use mystiko_relayer::application::run_application;
use std::path::Path;

pub const DEFAULT_SERVER_CONFIG_PATH: &str = "./config.toml";

#[actix_web::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let server_config_path = args
        .get(1)
        .map(|path| path.as_str())
        .unwrap_or(DEFAULT_SERVER_CONFIG_PATH);
    let path = if Path::new(server_config_path).try_exists()? {
        Some(server_config_path.to_string())
    } else {
        None
    };

    run_application(path).await
}
