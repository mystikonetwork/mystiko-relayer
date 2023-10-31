use anyhow::{bail, Result};
use config::FileFormat;
use log::debug;
use mystiko_relayer_config::wrapper::relayer::RelayerConfig;
use mystiko_types::NetworkType;
use mystiko_utils::config::{load_config, ConfigFile, ConfigLoadOptions};
use mystiko_validator::validate::is_api_version;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use validator::Validate;

#[derive(Validate, Serialize, Deserialize, Debug, Clone, Default)]
pub struct ServerConfig {
    #[validate]
    pub settings: Settings,
    #[validate]
    pub accounts: HashMap<u16, AccountConfig>,
    #[validate]
    #[serde(default)]
    pub options: Options,
}

#[derive(Validate, Serialize, Deserialize, Debug, Clone, Default)]
pub struct Settings {
    #[validate(custom(function = "is_api_version"))]
    pub api_version: HashMap<u16, String>,
    pub network_type: NetworkType,
    #[validate(contains = ".sqlite")]
    #[serde(default)]
    pub sqlite_db_path: Option<String>,
    pub log_level: String,
    pub host: String,
    pub port: u16,
    #[validate(length(min = 1))]
    pub coin_market_cap_api_key: String,
}

#[derive(Validate, Serialize, Deserialize, Debug, Clone, Default)]
pub struct AccountConfig {
    #[validate(range(min = 1))]
    pub chain_id: u64,
    pub private_key: String,
    pub available: bool,
    pub supported_erc20_tokens: HashMap<u16, String>,
    #[validate(range(min = 0.0001))]
    pub balance_alarm_threshold: f64,
    #[validate(range(min = 20000))]
    pub balance_check_interval_ms: u64,
}

#[derive(Validate, Serialize, Deserialize, Debug, Clone, Default)]
pub struct Options {
    #[serde(default)]
    pub mystiko_config_path: Option<String>,
    #[serde(default)]
    pub relayer_config_path: Option<String>,
    #[serde(default)]
    pub mystiko_remote_config_base_url: Option<String>,
    #[serde(default)]
    pub relayer_remote_config_base_url: Option<String>,
}

impl ServerConfig {
    pub fn find_accounts(&self, chain_id: u64) -> Option<Vec<&AccountConfig>> {
        let mut accounts = Vec::new();
        for account in self.accounts.values() {
            if account.chain_id == chain_id {
                accounts.push(account);
            }
        }
        if accounts.is_empty() {
            None
        } else {
            Some(accounts)
        }
    }

    pub fn find_accounts_available(&self, chain_id: u64) -> Option<Vec<&AccountConfig>> {
        self.find_accounts(chain_id)
            .map(|accounts| accounts.into_iter().filter(|account| account.available).collect())
    }

    pub fn validation(&self, relayer_config: &RelayerConfig) -> Result<()> {
        for account in self.accounts.values() {
            // validate account supported erc20 tokens
            let chain_config_opt = relayer_config.find_chain_config(account.chain_id);
            if chain_config_opt.is_none() {
                bail!("chain id {} not found in relayer config", account.chain_id);
            }
            let chain_config = chain_config_opt.unwrap();
            let symbols = chain_config
                .contracts()
                .iter()
                .map(|contract| contract.asset_symbol().to_lowercase())
                .collect::<Vec<String>>();
            debug!("chain id {} symbols {:?}", account.chain_id, symbols);
            debug!(
                "server config supported erc20 tokens {:?}",
                &account.supported_erc20_tokens
            );
            for tokens in account.supported_erc20_tokens.values() {
                if !symbols.contains(&tokens.to_lowercase()) {
                    bail!(
                        "chain_id {} token {} not found in relayer chain config",
                        account.chain_id,
                        tokens
                    );
                }
            }
        }
        Ok(self.validate()?)
    }
}

pub fn load_server_config(path: &str) -> Result<ServerConfig> {
    let format = FileFormat::Toml;
    let options = ConfigLoadOptions::builder()
        .paths(ConfigFile::builder().path(path).format(format).build())
        .env_prefix("RELAYER_CONFIG")
        .build();
    load_config::<PathBuf, ServerConfig>(&options)
}
