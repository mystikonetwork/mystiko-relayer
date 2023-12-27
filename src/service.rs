use crate::channel::transact_channel::find_producer_by_id_and_symbol;
use crate::channel::TransactSendersMap;
use crate::common::AppState;
use crate::error::ResponseError;
use crate::handler::account::handler::Account;
use crate::handler::account::AccountHandler;
use crate::handler::transaction::{Transaction, TransactionHandler};
use crate::mystiko_server_utils::tx_manager::TransactionMiddleware;
use actix_web::web::{Data, Json, Path};
use actix_web::{get, post, Responder};
use anyhow::{bail, Result};
use ethers_core::types::U256;
use ethers_signers::LocalWallet;
use log::{debug, error};
use mystiko_ethers::{ChainConfigProvidersOptions, ProviderPool, Providers};
use mystiko_relayer_config::wrapper::relayer::RelayerConfig;
use mystiko_relayer_types::response::success;
use mystiko_relayer_types::{
    ContractInfo, HandshakeResponse, RegisterInfoRequest, RegisterInfoResponse, RegisterOptions, RelayTransactResponse,
    RelayTransactStatusResponse, TransactRequestData,
};
use mystiko_server_utils::token_price::PriceMiddleware;
use mystiko_server_utils::token_price::TokenPrice;
use mystiko_server_utils::tx_manager::config::TxManagerConfig;
use mystiko_server_utils::tx_manager::TxManagerBuilder;
use mystiko_storage::SqlStatementFormatter;
use mystiko_storage_sqlite::SqliteStorage;
use mystiko_types::AssetType;
use std::ops::Mul;
use std::sync::Arc;
use tokio::sync::RwLock;
use validator::Validate;

#[get("/handshake")]
pub async fn handshake(data: Data<AppState>) -> actix_web::Result<impl Responder, ResponseError> {
    let api_version: Vec<String> = data.server_config.settings.api_version.values().cloned().collect();
    let package_version = env!("CARGO_PKG_VERSION");
    Ok(success(
        HandshakeResponse::builder()
            .package_version(String::from(package_version))
            .api_version(api_version)
            .build(),
    ))
}

#[post("/info")]
pub async fn info(
    request: Json<RegisterInfoRequest>,
    data: Data<AppState>,
    handler: Data<Arc<Account<SqlStatementFormatter, SqliteStorage>>>,
    token_price: Data<Arc<RwLock<TokenPrice>>>,
    providers: Data<Arc<ProviderPool<ChainConfigProvidersOptions>>>,
) -> actix_web::Result<impl Responder, ResponseError> {
    let relayer_config = &data.relayer_config;
    let chain_id = request.chain_id;

    // check relayer chain config and server config
    return if let Some(relayer_chain_config) = relayer_config.find_chain_config(chain_id) {
        let accounts = handler.find_by_chain_id(chain_id).await.map_err(|e| {
            error!("Failed to query accounts: {:?}", e);
            ResponseError::DatabaseError
        })?;

        if accounts.is_empty() {
            error!("account by chain id: {:?} not found", chain_id);
            return Err(ResponseError::AccountNotFoundInDatabase);
        }

        let account_supported_erc20_symbol = accounts
            .iter()
            .flat_map(|account| {
                account
                    .data
                    .supported_erc20_tokens
                    .iter()
                    .map(|symbol| symbol.to_lowercase())
            })
            .fold(Vec::new(), |mut acc, symbol| {
                let lowercase_symbol = symbol.to_lowercase();
                if !acc.contains(&lowercase_symbol) {
                    acc.push(lowercase_symbol);
                }
                acc
            });
        debug!(
            "chain_id: {}, account_supported_symbol: {:?}",
            chain_id, account_supported_erc20_symbol
        );

        // Check supported asset symbol
        if let Some(options) = &request.options {
            let asset_symbol_lowercase = &options.asset_symbol.to_lowercase();
            if !relayer_chain_config
                .asset_symbol()
                .eq_ignore_ascii_case(asset_symbol_lowercase)
                && !accounts
                    .iter()
                    .any(|account| account.data.supported_erc20_tokens.contains(asset_symbol_lowercase))
            {
                return Ok(success(
                    RegisterInfoResponse::builder()
                        .chain_id(chain_id)
                        .support(false)
                        .available(false)
                        .build(),
                ));
            }
        }

        // Check available
        if accounts.iter().all(|account| !account.data.available) {
            return Ok(success(
                RegisterInfoResponse::builder()
                    .chain_id(chain_id)
                    .support(true)
                    .available(false)
                    .build(),
            ));
        }

        let contracts_config = match &request.options {
            None => relayer_chain_config.contracts(),
            Some(options) => relayer_chain_config
                .find_contract(&options.asset_symbol)
                .map(|contract| vec![contract])
                .unwrap_or_default(),
        };
        let mut contracts: Vec<ContractInfo> = Vec::new();
        for contract in contracts_config {
            let lowercase_symbol = &contract.asset_symbol().to_lowercase();
            if !account_supported_erc20_symbol.contains(lowercase_symbol)
                && !relayer_chain_config
                    .asset_symbol()
                    .eq_ignore_ascii_case(lowercase_symbol)
            {
                continue;
            }
            let minimum_gas_fee = if let Some(options) = &request.options {
                let gas_price = gas_price_by_chain_id(chain_id, providers.clone()).await;
                if gas_price.is_err() {
                    error!("get chain id {} gas price error {}", chain_id, gas_price.unwrap_err());
                    return Err(ResponseError::GetGasPriceError { chain_id });
                }
                let gas_price = gas_price.unwrap();
                debug!("chain id {} gas prices {:?}", chain_id, gas_price);

                let minimum_gas_fee =
                    minimum_gas_fee(&data.relayer_config, chain_id, gas_price, token_price.clone(), options).await;
                if minimum_gas_fee.is_err() {
                    error!("Failed to get minimum gas fee: {:?}", minimum_gas_fee.unwrap_err());
                    return Err(ResponseError::GetMinimumGasFeeFailed);
                }
                Some(minimum_gas_fee.unwrap())
            } else {
                None
            };
            contracts.push(
                ContractInfo::builder()
                    .asset_symbol(contract.asset_symbol().to_string())
                    .relayer_fee_of_ten_thousandth(contract.relayer_fee_of_ten_thousandth())
                    .minimum_gas_fee(minimum_gas_fee.map(|minimum_gas_fee| minimum_gas_fee.to_string()))
                    .build(),
            );
        }
        Ok(success(
            RegisterInfoResponse::builder()
                .chain_id(chain_id)
                .support(true)
                .available(true)
                .relayer_contract_address(String::from(relayer_chain_config.relayer_contract_address()))
                .contracts(contracts)
                .build(),
        ))
    } else {
        Ok(success(
            RegisterInfoResponse::builder()
                .chain_id(chain_id)
                .support(false)
                .available(false)
                .build(),
        ))
    };
}

#[post("/transact")]
pub async fn transact(
    request: Json<TransactRequestData>,
    data: Data<AppState>,
    senders: Data<TransactSendersMap>,
    handler: Data<Arc<Transaction<SqlStatementFormatter, SqliteStorage>>>,
) -> actix_web::Result<impl Responder, ResponseError> {
    // validate
    if let Err(err) = request.validate() {
        error!("transact request body validate error {:?}", err);
        return Err(ResponseError::ValidateError { error: err.to_string() });
    }

    // check repeated transaction
    if let Ok(repeat) = handler.is_repeated_transaction(&request.signature).await {
        if repeat {
            return Err(ResponseError::RepeatedTransaction);
        }
    } else {
        return Err(ResponseError::DatabaseError);
    }

    let chain_config = &data.relayer_config.find_chain_config(request.chain_id);
    if chain_config.is_none() {
        return Err(ResponseError::ChainIdNotFoundInRelayerConfig {
            chain_id: request.chain_id,
        });
    }
    let chain_config = chain_config.unwrap();

    let asset_type = if chain_config.asset_symbol().eq_ignore_ascii_case(&request.asset_symbol) {
        AssetType::Main
    } else {
        AssetType::Erc20
    };

    // save data and sent
    match find_producer_by_id_and_symbol(&senders, request.chain_id, &request.asset_symbol, asset_type) {
        Some(producer) => match producer.send(request.into_inner()).await {
            Ok(transaction) => Ok(success(RelayTransactResponse { uuid: transaction.id })),
            Err(error) => {
                error!("send transact request to queue got error: {:?}", error);
                Err(ResponseError::TransactionChannelError {
                    error: error.to_string(),
                })
            }
        },
        None => Err(ResponseError::UnsupportedTransaction),
    }
}

#[get("/transaction/status/{id}")]
pub async fn transaction_status(
    id: Path<String>,
    handler: Data<Arc<Transaction<SqlStatementFormatter, SqliteStorage>>>,
) -> actix_web::Result<impl Responder, ResponseError> {
    match handler.find_by_id(id.as_str()).await {
        Ok(Some(transaction)) => Ok(success(
            RelayTransactStatusResponse::builder()
                .uuid(transaction.id)
                .chain_id(transaction.data.chain_id)
                .spend_type(transaction.data.spend_type)
                .status(transaction.data.status)
                .transaction_hash(transaction.data.transaction_hash)
                .error_msg(transaction.data.error_message)
                .build(),
        )),
        Ok(None) => Err(ResponseError::TransactionNotFound { id: id.into_inner() }),
        Err(error) => {
            error!("find transaction by id({}) got error: {:?}", id, error);
            Err(ResponseError::DatabaseError)
        }
    }
}

pub async fn minimum_gas_fee(
    relayer_config: &RelayerConfig,
    chain_id: u64,
    gas_price: U256,
    token: Data<Arc<RwLock<TokenPrice>>>,
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

async fn gas_price_by_chain_id<P: Providers>(chain_id: u64, providers: Data<Arc<P>>) -> Result<U256> {
    let provider = providers.get_provider(chain_id).await?;
    let tx_builder = TxManagerBuilder::builder()
        .config(TxManagerConfig::new(None)?)
        .chain_id(chain_id)
        .wallet(LocalWallet::new(&mut rand::thread_rng()))
        .build();
    let tx_manager = tx_builder.build(&provider).await?;
    Ok(tx_manager.gas_price(&provider).await?)
}
