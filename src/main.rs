use anyhow::Result;
use mystiko_relayer::application::{run_application, ApplicationOptions};
use mystiko_relayer::configs::load_server_config;
use mystiko_storage::SqlStatementFormatter;
use mystiko_storage_sqlite::SqliteStorage;
use std::path::Path;
use std::sync::Arc;

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

    // init server config
    let server_config = Arc::new(load_server_config(path.as_deref())?);
    let options: ApplicationOptions<SqlStatementFormatter, SqliteStorage> =
        ApplicationOptions::<SqlStatementFormatter, SqliteStorage>::from_server_config(server_config).await?;

    run_application(options).await
}
