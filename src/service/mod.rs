pub mod v1;
pub mod v2;

use crate::channel::producer::handler::TransactionProducer;
use crate::channel::SenderInfo;
use crate::context::Context;
use crate::error::ResponseError;
use actix_web::web::Data;
use actix_web::{get, Responder};
use anyhow::bail;
use anyhow::Result;
use ethereum_types::U256;
use ethers_signers::LocalWallet;
use log::debug;
use mystiko_ethers::Providers;
use mystiko_relayer_config::wrapper::relayer::RelayerConfig;
use mystiko_relayer_types::response::success;
use mystiko_relayer_types::HandshakeResponse;
use mystiko_relayer_types::RegisterOptions;
use mystiko_server_utils::token_price::{PriceMiddleware, TokenPrice};
use mystiko_server_utils::tx_manager::config::TxManagerConfig;
use mystiko_server_utils::tx_manager::{TransactionMiddleware, TxManagerBuilder};
use mystiko_types::AssetType;
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::collections::HashSet;
use std::ops::Mul;
use std::sync::Arc;
use tokio::sync::RwLock;

#[get("/handshake")]
pub async fn handshake(context: Data<Arc<Context>>) -> actix_web::Result<impl Responder, ResponseError> {
    let api_version: Vec<String> = context.server_config.settings.api_version.values().cloned().collect();
    let package_version = env!("CARGO_PKG_VERSION");
    Ok(success(
        HandshakeResponse::builder()
            .package_version(String::from(package_version))
            .api_version(api_version)
            .build(),
    ))
}

pub fn find_sender(
    senders: Data<Arc<HashSet<SenderInfo>>>,
    chain_id: u64,
    asset_symbol: &str,
    asset_type: AssetType,
) -> Option<Arc<TransactionProducer>> {
    let matches = senders
        .iter()
        .filter(|sender| {
            if chain_id != sender.chain_id {
                return false;
            }
            if asset_type == AssetType::Main {
                return true;
            }
            let contains = sender
                .supported_erc20_tokens
                .iter()
                .map(|symbol| symbol.to_lowercase())
                .any(|symbol| symbol == asset_symbol.to_lowercase());
            contains
        })
        .collect::<Vec<_>>();

    // Select one at random and return
    let mut rng = thread_rng();
    if let Some(sender) = matches.choose(&mut rng) {
        return Some(sender.producer.clone());
    }

    None
}

async fn gas_price_by_chain_id<P: Providers>(chain_id: u64, providers: Arc<P>, is_tx_eip1559: bool) -> Result<U256> {
    let provider = providers.get_provider(chain_id).await?;
    let tx_builder = TxManagerBuilder::builder()
        .config(TxManagerConfig::new(None)?)
        .chain_id(chain_id)
        .wallet(LocalWallet::new(&mut rand::thread_rng()))
        .build();
    let tx_manager = tx_builder.build(Some(is_tx_eip1559), &provider).await?;
    Ok(tx_manager.gas_price(&provider).await?)
}

async fn minimum_gas_fee(
    relayer_config: &RelayerConfig,
    chain_id: u64,
    gas_price: U256,
    token: Arc<RwLock<TokenPrice>>,
    options: &RegisterOptions,
) -> Result<U256> {
    let asset_symbol = &options.asset_symbol;
    let circuit_type = &options.circuit_type;

    let relayer_chain_config = relayer_config.find_chain_config(chain_id);
    if relayer_chain_config.is_none() {
        bail!("chain id {} config not found in relayer config", chain_id)
    }
    let relayer_chain_config = relayer_chain_config.unwrap();

    let contract_config = relayer_chain_config.find_contract(asset_symbol);
    if contract_config.is_none() {
        bail!(
            "asset symbol {} contract config not found in chain id {} config",
            asset_symbol,
            chain_id
        )
    }
    let contract_config = contract_config.unwrap();

    let main_asset_symbol = relayer_chain_config.asset_symbol();
    let main_asset_decimals = relayer_chain_config.asset_decimals();
    debug!(
        "chain id {}, main asset symbol {}, main asset decimals {}",
        chain_id, main_asset_symbol, main_asset_decimals
    );

    let asset_type: &AssetType = contract_config.asset_type();
    let asset_decimals = contract_config.asset_decimals();
    debug!(
        "asset symbol {} asset type {:?}, asset decimals {}",
        asset_symbol, asset_type, asset_decimals
    );

    let gas_cost = relayer_chain_config.find_gas_cost(asset_type, circuit_type)?;
    debug!("circuit type {:?} gas cost {}", circuit_type, gas_cost);

    let minimum_gas_fee = gas_price.mul(gas_cost);

    match asset_type {
        AssetType::Erc20 => {
            // swap main to erc20
            let token_price = token.write().await;
            let result = token_price
                .swap(
                    main_asset_symbol,
                    main_asset_decimals,
                    minimum_gas_fee,
                    asset_symbol,
                    asset_decimals,
                )
                .await?;
            drop(token_price);
            Ok(result)
        }
        AssetType::Main => Ok(minimum_gas_fee),
    }
}
