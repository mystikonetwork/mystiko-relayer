pub mod consumer;
pub mod producer;

use crate::channel::producer::TransactionProducer;
use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct TransactSendersMapKey {
    pub chain_id: u64,
    pub private_key: String,
}

pub type TransactSendersMap = HashMap<TransactSendersMapKey, TransactionProducer>;

pub mod transact_channel {
    use crate::channel::consumer::TransactionConsumer;
    use crate::channel::producer::TransactionProducer;
    use crate::channel::{TransactSendersMap, TransactSendersMapKey};
    use crate::configs::ServerConfig;
    use crate::handler::transaction::TransactionHandler;
    use anyhow::{bail, Result};
    use ethers_signers::{LocalWallet, Signer};
    use mystiko_config::MystikoConfig;
    use mystiko_ethers::{
        DefaultProviderFactory, Provider, ProviderFactory, ProviderOptions, Providers, ProvidersOptions, HTTP_REGEX,
        WS_REGEX,
    };
    use mystiko_relayer_config::wrapper::relayer::RelayerConfig;
    use mystiko_relayer_types::TransactRequestData;
    use mystiko_server_utils::token_price::TokenPrice;
    use mystiko_server_utils::tx_manager::config::TxManagerConfig;
    use mystiko_server_utils::tx_manager::TxManagerBuilder;
    use mystiko_storage::SqlStatementFormatter;
    use mystiko_storage_sqlite::SqliteStorage;
    use mystiko_types::AssetType;
    use rand::seq::SliceRandom;
    use rand::thread_rng;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::mpsc::channel;
    use tokio::sync::RwLock;

    pub async fn init<P: Providers>(
        server_config: &ServerConfig,
        relayer_config: &RelayerConfig,
        mystiko_config: &MystikoConfig,
        providers: Arc<P>,
        handler: Arc<TransactionHandler<SqlStatementFormatter, SqliteStorage>>,
        token_price: Arc<RwLock<TokenPrice>>,
        queue_capacity: usize,
    ) -> Result<(TransactSendersMap, Vec<TransactionConsumer<P>>)> {
        let mut transact_senders_map = HashMap::new();
        let mut consumers = Vec::new();
        for account in server_config.accounts.values() {
            let chain_id = account.chain_id;
            let pk = &account.private_key;
            let (sender, receiver) = channel::<(String, TransactRequestData)>(queue_capacity);
            transact_senders_map.insert(
                TransactSendersMapKey {
                    chain_id,
                    private_key: pk.to_string(),
                },
                TransactionProducer::new(
                    account.supported_erc20_tokens.values().cloned().collect(),
                    Arc::new(sender),
                    handler.clone(),
                ),
            );
            let wallet: LocalWallet = pk.parse::<LocalWallet>()?.with_chain_id(chain_id);

            // create tx manager config
            let tx_manager_config = TxManagerConfig::new(None)?;
            // create tx builder
            let tx_builder = TxManagerBuilder::builder()
                .config(tx_manager_config)
                .chain_id(chain_id)
                .wallet(wallet)
                .build();
            // get or create provider
            let provider = providers.get_provider(chain_id).await?;
            // build tx manager
            let tx_manager = tx_builder.build(&provider).await?;

            // create signer provider
            let mystiko_chain_config = mystiko_config
                .find_chain(chain_id)
                .unwrap_or_else(|| panic!("chain id {} config not found in mystiko config", chain_id));
            let signer_endpoint = mystiko_chain_config.signer_endpoint();
            let signer_provider = create_signer_provider(signer_endpoint).await?;

            // found relayer chain config
            let relayer_chain_config = relayer_config
                .find_chain_config(chain_id)
                .unwrap_or_else(|| panic!("chain id {} config not found in relayer config", chain_id));

            consumers.push(TransactionConsumer {
                chain_id,
                main_asset_symbol: String::from(relayer_chain_config.asset_symbol()),
                main_asset_decimals: relayer_chain_config.asset_decimals(),
                receiver,
                providers: providers.clone(),
                signer: Arc::new(signer_provider),
                handler: handler.clone(),
                token_price: token_price.clone(),
                tx_manager,
            });
        }

        Ok((transact_senders_map, consumers))
    }

    pub async fn create_signer_provider(url: &str) -> Result<Provider> {
        let option = ProviderOptions::builder().url(url.to_string()).build();
        if HTTP_REGEX.is_match(url) {
            let options = ProvidersOptions::Http(option);
            DefaultProviderFactory::new().create_provider(options).await
        } else if WS_REGEX.is_match(url) {
            let options = ProvidersOptions::Ws(option);
            DefaultProviderFactory::new().create_provider(options).await
        } else {
            bail!("url {} signer endpoint is not valid", url)
        }
    }

    pub fn find_producer_by_id_and_symbol(
        senders: &TransactSendersMap,
        chain_id: u64,
        asset_symbol: &str,
        asset_type: AssetType,
    ) -> Option<TransactionProducer> {
        let matching_producers = senders
            .iter()
            .filter(|(key, value)| {
                if key.chain_id != chain_id {
                    return false;
                }
                if asset_type == AssetType::Main {
                    return true;
                }
                let contains = value
                    .supported_erc20_tokens
                    .iter()
                    .map(|symbol| symbol.to_lowercase())
                    .any(|symbol| symbol == asset_symbol.to_lowercase());
                contains
            })
            .map(|(_, value)| value.clone())
            .collect::<Vec<_>>();

        // Select one at random and return
        let mut rng = thread_rng();
        if let Some(sender) = matching_producers.choose(&mut rng) {
            return Some(sender.clone());
        }

        None
    }
}
