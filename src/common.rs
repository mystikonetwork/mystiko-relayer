use crate::configs::ServerConfig;
use anyhow::Result;
use mystiko_config::MystikoConfig;
use mystiko_protos::common::v1::ConfigOptions;
use mystiko_relayer_config::wrapper::relayer::RelayerConfig;
use mystiko_types::NetworkType::Testnet;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct AppState {
    pub server_config: Arc<ServerConfig>,
    pub relayer_config: Arc<RelayerConfig>,
    pub mystiko_config: Arc<MystikoConfig>,
}

pub async fn init_app_state(server_config: ServerConfig) -> Result<AppState> {
    let relayer_config_path = &server_config.options.relayer_config_path;
    let mystiko_config_path = &server_config.options.mystiko_config_path;

    // load default relayer config
    let relayer_config = match relayer_config_path {
        None => {
            let mut options = if let Some(base_url) = &server_config.options.relayer_remote_config_base_url {
                mystiko_relayer_config::wrapper::relayer::RemoteOptions::builder()
                    .base_url(base_url.to_string())
                    .build()
            } else {
                mystiko_relayer_config::wrapper::relayer::RemoteOptions::builder().build()
            };
            if server_config.settings.network_type == Testnet {
                options.is_testnet = true;
            }
            RelayerConfig::from_remote(&options).await?
        }
        Some(path) => RelayerConfig::from_json_file(path).await?,
    };

    // load default mystiko config
    let mystiko_config = match mystiko_config_path {
        None => {
            let mut options = if let Some(base_url) = &server_config.options.mystiko_remote_config_base_url {
                ConfigOptions::builder().remote_base_url(base_url.to_string()).build()
            } else {
                ConfigOptions::builder().build()
            };
            if server_config.settings.network_type == Testnet {
                options.is_testnet = Some(true);
            }
            MystikoConfig::from_remote(&options).await?
        }
        Some(path) => MystikoConfig::from_json_file(path).await?,
    };

    // validation config
    server_config.validation(&relayer_config)?;

    Ok(AppState {
        server_config: Arc::new(server_config),
        relayer_config: Arc::new(relayer_config),
        mystiko_config: Arc::new(mystiko_config),
    })
}
