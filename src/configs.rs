use anyhow::{bail, Result};
use config::FileFormat;
use dotenv::dotenv;
use log::debug;
use mystiko_relayer_config::wrapper::relayer::RelayerConfig;
use mystiko_types::NetworkType;
use mystiko_utils::config::{load_config, ConfigFile, ConfigLoadOptions};
use mystiko_validator::validate::is_api_version;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use typed_builder::TypedBuilder;
use validator::Validate;

#[derive(TypedBuilder, Validate, Serialize, Deserialize, Debug, Clone)]
#[builder(field_defaults(setter(into)))]
pub struct ServerConfig {
    #[validate]
    #[serde(default)]
    #[builder(default)]
    pub settings: Settings,
    #[validate]
    #[serde(default)]
    #[builder(default)]
    pub accounts: HashMap<u16, AccountConfig>,
    #[validate]
    #[serde(default)]
    #[builder(default)]
    pub options: Options,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self::builder().build()
    }
}

#[derive(TypedBuilder, Validate, Serialize, Deserialize, Debug, Clone)]
#[builder(field_defaults(setter(into)))]
pub struct Settings {
    #[builder(default)]
    #[validate(custom(function = "is_api_version"))]
    pub api_version: HashMap<u16, String>,
    #[builder(default)]
    pub network_type: NetworkType,
    #[serde(default)]
    #[builder(default)]
    #[validate(contains = ".sqlite")]
    pub sqlite_db_path: Option<String>,
    #[serde(default = "default_log_level")]
    #[builder(default = default_log_level())]
    pub log_level: String,
    #[serde(default = "default_host")]
    #[builder(default = default_host())]
    pub host: String,
    #[serde(default = "default_port")]
    #[builder(default = default_port())]
    pub port: u16,
    #[builder(default)]
    #[validate(length(min = 1))]
    pub coin_market_cap_api_key: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self::builder().build()
    }
}

#[derive(TypedBuilder, Validate, Serialize, Deserialize, Debug, Clone)]
#[builder(field_defaults(setter(into)))]
pub struct AccountConfig {
    #[builder(default)]
    #[validate(range(min = 1))]
    pub chain_id: u64,
    #[builder(default)]
    pub private_key: String,
    #[serde(default = "default_available")]
    #[builder(default = default_available())]
    pub available: bool,
    #[builder(default)]
    pub supported_erc20_tokens: HashMap<u16, String>,
    #[serde(default)]
    #[builder(default)]
    pub balance_alarm_threshold: f64,
    #[serde(default)]
    #[builder(default)]
    pub balance_check_interval_ms: u64,
}

impl Default for AccountConfig {
    fn default() -> Self {
        Self::builder().build()
    }
}

#[derive(TypedBuilder, Validate, Serialize, Deserialize, Debug, Clone)]
#[builder(field_defaults(setter(into)))]
pub struct Options {
    #[serde(default)]
    #[builder(default)]
    pub mystiko_config_path: Option<String>,
    #[serde(default)]
    #[builder(default)]
    pub relayer_config_path: Option<String>,
    #[serde(default)]
    #[builder(default)]
    pub mystiko_remote_config_base_url: Option<String>,
    #[serde(default)]
    #[builder(default)]
    pub relayer_remote_config_base_url: Option<String>,
}

impl Default for Options {
    fn default() -> Self {
        Self::builder().build()
    }
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

fn default_log_level() -> String {
    "info".to_string()
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    8090
}

fn default_available() -> bool {
    true
}
