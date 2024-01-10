use crate::configs::account::AccountConfig;
use crate::configs::server::ServerConfig;
use crate::database::account::Account as DocumentAccount;
use crate::database::transaction::Transaction as DocumentTransaction;
use crate::database::Database;
use crate::error::RelayerServerError;
use crate::handler::account::handler::Account;
use crate::handler::account::AccountHandler;
use crate::handler::transaction::{Transaction, TransactionHandler};
use anyhow::Result;
use mystiko_config::MystikoConfig;
use mystiko_ethers::{ChainConfigProvidersOptions, ProviderPool, Providers};
use mystiko_protos::common::v1::ConfigOptions;
use mystiko_relayer_config::wrapper::relayer::RelayerConfig;
use mystiko_server_utils::token_price::config::TokenPriceConfig;
use mystiko_server_utils::token_price::{PriceMiddleware, TokenPrice};
use mystiko_storage::{Document, SqlStatementFormatter};
use mystiko_storage_sqlite::SqliteStorage;
use mystiko_types::NetworkType;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone, Debug)]
pub struct Context {
    pub server_config: Arc<ServerConfig>,
    pub relayer_config: Arc<RelayerConfig>,
    pub mystiko_config: Arc<MystikoConfig>,
    pub providers: Arc<Box<dyn Providers>>,
    pub transaction_handler:
        Arc<Box<dyn TransactionHandler<Document<DocumentTransaction>, Error = RelayerServerError>>>,
    pub account_handler: Arc<Box<dyn AccountHandler<Document<DocumentAccount>, Error = RelayerServerError>>>,
    pub token_price: Arc<RwLock<Box<dyn PriceMiddleware>>>,
}

impl Context {
    pub async fn new(
        server_config: Arc<ServerConfig>,
        database: Database<SqlStatementFormatter, SqliteStorage>,
    ) -> Result<Self> {
        let db = Arc::new(database);

        // create relayer and mystiko config
        let (relayer_config, mystiko_config) = create_config(server_config.clone()).await?;

        // validation server config
        server_config.validation(&relayer_config)?;

        // create provider
        let providers: ProviderPool<ChainConfigProvidersOptions> = mystiko_config.clone().into();
        let providers = Arc::new(Box::new(providers) as Box<dyn Providers>);

        // create transaction handler
        let transaction_handler = Transaction::new(db.clone());
        let transaction_handler = Arc::new(Box::new(transaction_handler)
            as Box<dyn TransactionHandler<Document<DocumentTransaction>, Error = RelayerServerError>>);

        // create account handler
        let account_handler = Account::new(
            db.clone(),
            server_config
                .accounts
                .values()
                .cloned()
                .collect::<Vec<AccountConfig>>()
                .as_slice(),
        )
        .await?;
        let account_handler =
            Arc::new(Box::new(account_handler)
                as Box<
                    dyn AccountHandler<Document<DocumentAccount>, Error = RelayerServerError>,
                >);

        // init token price
        let token_price = Arc::new(RwLock::new(Box::new(TokenPrice::new(
            &TokenPriceConfig::new(
                serde_json::to_string(&server_config.settings.network_type)?.as_str(),
                None,
            )?,
            &server_config.settings.coin_market_cap_api_key,
        )?) as Box<dyn PriceMiddleware>));

        Ok(Self {
            server_config,
            relayer_config,
            mystiko_config,
            providers,
            transaction_handler,
            account_handler,
            token_price,
        })
    }
}

async fn create_config(server_config: Arc<ServerConfig>) -> Result<(Arc<RelayerConfig>, Arc<MystikoConfig>)> {
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
            if server_config.settings.network_type == NetworkType::Testnet {
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
            if server_config.settings.network_type == NetworkType::Testnet {
                options.is_testnet = Some(true);
            }
            MystikoConfig::from_remote(&options).await?
        }
        Some(path) => MystikoConfig::from_json_file(path).await?,
    };

    Ok((Arc::new(relayer_config), Arc::new(mystiko_config)))
}
