pub mod account;
pub mod server;

use crate::configs::server::ServerConfig;
use anyhow::Result;
use config::FileFormat;
use dotenv::dotenv;
use mystiko_utils::config::{load_config, ConfigFile, ConfigLoadOptions};
use std::path::PathBuf;

pub fn load_server_config(path: Option<&str>) -> Result<ServerConfig> {
    let options = if let Some(path) = path {
        let format = FileFormat::Toml;
        ConfigLoadOptions::builder()
            .paths(ConfigFile::builder().path(path).format(format).build())
            .env_prefix("RELAYER_CONFIG")
            .build()
    } else {
        dotenv().ok();
        ConfigLoadOptions::builder().env_prefix("MYSTIKO_RELAYER").build()
    };
    load_config::<PathBuf, ServerConfig>(&options)
}
