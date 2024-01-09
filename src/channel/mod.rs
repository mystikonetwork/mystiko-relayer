use crate::channel::consumer::handler::TransactionConsumer;
use crate::channel::producer::handler::TransactionProducer;
use crate::context::Context;
use anyhow::Result;
use ethers_signers::{LocalWallet, Signer};
use mystiko_ethers::Providers;
use mystiko_relayer_types::TransactRequestData;
use mystiko_server_utils::tx_manager::config::TxManagerConfig;
use mystiko_server_utils::tx_manager::TxManagerBuilder;
use mystiko_types::TransactionType;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use tokio::sync::mpsc::channel;

pub mod consumer;
pub mod producer;

pub const ARRAY_QUEUE_CAPACITY: usize = 50;

#[derive(Debug)]
pub struct SenderInfo {
    pub chain_id: u64,
    pub private_key: String,
    pub supported_erc20_tokens: Vec<String>,
    pub producer: Arc<TransactionProducer>,
}

impl PartialEq<Self> for SenderInfo {
    fn eq(&self, other: &Self) -> bool {
        self.chain_id == other.chain_id && self.private_key == other.private_key
    }
}

impl Eq for SenderInfo {}

impl Hash for SenderInfo {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.chain_id.hash(state);
        self.private_key.hash(state);
    }
}

pub struct Channel {
    pub senders: HashSet<SenderInfo>,
    pub consumers: Vec<TransactionConsumer>,
}

impl Channel {
    pub async fn new(context: Arc<Context>) -> Result<Self> {
        let mut senders = HashSet::new();
        let mut consumers: Vec<TransactionConsumer> = Vec::new();
        for account in context.server_config.accounts.values() {
            let chain_id = account.chain_id;
            let private_key = &account.private_key;
            let supported_erc20_tokens = account.supported_erc20_tokens.values().cloned().collect();
            let chain_config = context
                .mystiko_config
                .find_chain(chain_id)
                .unwrap_or_else(|| panic!("chain id {} config not found in mystiko config", chain_id));
            let is_tx_eip1559 = chain_config.transaction_type() == &TransactionType::Eip1559;
            let (sender, receiver) = channel::<(String, TransactRequestData)>(ARRAY_QUEUE_CAPACITY);
            let producer = Arc::new(TransactionProducer::new(
                Arc::new(sender),
                context.transaction_handler.clone(),
            ));
            senders.insert(SenderInfo {
                chain_id,
                supported_erc20_tokens,
                producer,
                private_key: private_key.to_string(),
            });

            let wallet: LocalWallet = private_key.parse::<LocalWallet>()?.with_chain_id(chain_id);

            // create tx manager
            let tx_manager_config = TxManagerConfig::new(None)?;
            // create tx builder
            let tx_builder = TxManagerBuilder::builder()
                .config(tx_manager_config)
                .chain_id(chain_id)
                .wallet(wallet)
                .build();
            // get or create provider
            let provider = context.providers.get_provider(chain_id).await?;
            // build tx manager
            let tx_manager = tx_builder.build(Some(is_tx_eip1559), &provider).await?;

            // found relayer chain config
            let relayer_chain_config = context
                .relayer_config
                .find_chain_config(chain_id)
                .unwrap_or_else(|| panic!("chain id {} config not found in relayer config", chain_id));
            consumers.push(TransactionConsumer {
                chain_id,
                is_tx_eip1559,
                main_asset_symbol: relayer_chain_config.asset_symbol().to_string(),
                main_asset_decimals: relayer_chain_config.asset_decimals(),
                receiver,
                providers: context.providers.clone(),
                handler: context.transaction_handler.clone(),
                token_price: context.token_price.clone(),
                tx_manager,
            });
        }

        Ok(Self { senders, consumers })
    }
}
