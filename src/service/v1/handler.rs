use crate::channel::producer::ProducerHandler;
use crate::channel::SenderInfo;
use crate::context::Context;
use crate::error::ResponseError;
use crate::service::v1::parse_transact_request;
use crate::service::v1::request::{ChainStatusRequest, TransactRequestV1};
use crate::service::v1::response::{
    ChainStatusResponse, ContractResponse, JobStatusResponse, ResponseQueueData, TransactResponse,
};
use crate::service::{find_sender, gas_price_by_chain_id, minimum_gas_fee};
use actix_web::web::{Data, Json, Path};
use actix_web::{get, post, Responder};
use log::{debug, error, info};
use mystiko_relayer_types::response::success;
use mystiko_relayer_types::{RegisterOptions, TransactStatus};
use mystiko_types::{AssetType, TransactionType};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use validator::Validate;

#[post("status")]
pub async fn chain_status(
    request: Json<ChainStatusRequest>,
    context: Data<Arc<Context>>,
) -> actix_web::Result<impl Responder, ResponseError> {
    info!("api v1 version chain status");

    let chain_id = request.chain_id;
    let relayer_config = &context.relayer_config;
    let mystiko_config = &context.mystiko_config;
    let handler = &context.account_handler;
    let token_price = &context.token_price;
    let providers = &context.providers;

    let is_tx_eip1559 = match mystiko_config.find_chain(chain_id) {
        None => {
            return Ok(success(ChainStatusResponse {
                support: false,
                available: false,
                chain_id,
                relayer_contract_address: None,
                contracts: None,
            }));
        }
        Some(chain_config) => chain_config.transaction_type() == &TransactionType::Eip1559,
    };

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
                return Ok(success(ChainStatusResponse {
                    support: false,
                    available: false,
                    chain_id,
                    relayer_contract_address: None,
                    contracts: None,
                }));
            }
        }

        // Check available
        if accounts.iter().all(|account| !account.data.available) {
            return Ok(success(ChainStatusResponse {
                support: true,
                available: false,
                chain_id,
                relayer_contract_address: None,
                contracts: None,
            }));
        }

        let contracts_config = match &request.options {
            None => relayer_chain_config.contracts(),
            Some(options) => relayer_chain_config
                .find_contract(&options.asset_symbol)
                .map(|contract| vec![contract])
                .unwrap_or_default(),
        };
        let mut contracts: Vec<ContractResponse> = Vec::new();
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
                let gas_price = gas_price_by_chain_id(chain_id, providers.clone(), is_tx_eip1559).await;
                if gas_price.is_err() {
                    return Err(ResponseError::GetGasPriceError { chain_id });
                }
                let gas_price = gas_price.unwrap();
                debug!("chain id {} gas prices {:?}", chain_id, gas_price);

                let minimum_gas_fee = minimum_gas_fee(
                    &relayer_config,
                    chain_id,
                    gas_price,
                    token_price.clone(),
                    &RegisterOptions {
                        asset_symbol: options.asset_symbol.to_string(),
                        circuit_type: options.circuit_type,
                        show_unavailable: false,
                    },
                )
                .await;
                if minimum_gas_fee.is_err() {
                    error!("Failed to get minimum gas fee: {:?}", minimum_gas_fee.unwrap_err());
                    return Err(ResponseError::GetMinimumGasFeeFailed);
                }
                Some(minimum_gas_fee.unwrap())
            } else {
                None
            };
            contracts.push(ContractResponse {
                asset_symbol: contract.asset_symbol().to_string(),
                relayer_fee_of_ten_thousandth: contract.relayer_fee_of_ten_thousandth(),
                minimum_gas_fee: minimum_gas_fee.map(|minimum_gas_fee| minimum_gas_fee.to_string()),
            });
        }
        Ok(success(ChainStatusResponse {
            support: true,
            available: true,
            chain_id,
            relayer_contract_address: Some(String::from(relayer_chain_config.relayer_contract_address())),
            contracts: Some(contracts),
        }))
    } else {
        Ok(success(ChainStatusResponse {
            support: false,
            available: false,
            chain_id,
            relayer_contract_address: None,
            contracts: None,
        }))
    };
}

#[get("/jobs/{id}")]
pub async fn job_status(
    id: Path<String>,
    context: Data<Arc<Context>>,
) -> actix_web::Result<impl Responder, ResponseError> {
    info!("api v1 version job status");

    let handler = &context.transaction_handler;

    match handler.find_by_id(id.as_str()).await {
        Ok(Some(transaction)) => Ok(success(JobStatusResponse {
            id: transaction.id,
            job_type: transaction.data.spend_type,
            status: transaction.data.status,
            response: transaction.data.transaction_hash.map(|hash| ResponseQueueData {
                hash,
                chain_id: transaction.data.chain_id,
            }),
            error: transaction.data.error_message,
        })),
        Ok(None) => Err(ResponseError::TransactionNotFound { id: id.into_inner() }),
        Err(error) => {
            error!("find transaction by id({}) got error: {:?}", id, error);
            Err(ResponseError::DatabaseError)
        }
    }
}

#[post("transact")]
pub async fn transact_v1(
    request: Json<TransactRequestV1>,
    context: Data<Arc<Context>>,
    senders: Data<Arc<HashSet<SenderInfo>>>,
) -> actix_web::Result<impl Responder, ResponseError> {
    info!("api v1 version transact");

    let relayer_config = &context.relayer_config;
    let mystiko_config = &context.mystiko_config;
    let handler = &context.transaction_handler;

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

    let chain_config = &relayer_config.find_chain_config(request.chain_id);
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

    let pool_contract = mystiko_config.find_pool_contract_by_address(request.chain_id, request.pool_address.as_str());
    if pool_contract.is_none() {
        return Err(ResponseError::Unknown);
    }
    let pool_contract = pool_contract.unwrap();

    let request = parse_transact_request(request.into_inner(), pool_contract.asset_decimals()).map_err(|err| {
        error!("parse transact request error {:?}", err);
        ResponseError::Unknown
    })?;

    // save data and sent
    match find_sender(senders, request.chain_id, &request.asset_symbol, asset_type) {
        None => Err(ResponseError::UnsupportedTransaction),
        Some(producer) => match producer.send(request).await {
            Ok(transaction) => {
                let mut response = TransactResponse {
                    id: transaction.id.to_string(),
                    hash: "".to_string(),
                    chain_id: transaction.data.chain_id,
                };
                loop {
                    // wait transaction hash
                    let transaction = handler.find_by_id(transaction.id.as_str()).await.map_err(|error| {
                        error!("find transaction by id({}) got error: {:?}", transaction.id, error);
                        ResponseError::TransactionNotFound {
                            id: transaction.id.to_string(),
                        }
                    })?;
                    match transaction {
                        None => {
                            info!("transaction not found, continue wait");
                        }
                        Some(doc) => {
                            if doc.data.status == TransactStatus::Failed {
                                return Err(ResponseError::TransactionFailed {
                                    error: doc.data.error_message.unwrap_or("unknown".to_string()),
                                });
                            }
                            match doc.data.transaction_hash {
                                None => {
                                    info!("transaction hash not found, continue wait");
                                }
                                Some(hash) => {
                                    response.hash = hash;
                                    break;
                                }
                            }
                        }
                    }
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }

                Ok(success(response))
            }
            Err(error) => {
                error!("send transact request to queue got error: {:?}", error);
                Err(ResponseError::TransactionChannelError {
                    error: error.to_string(),
                })
            }
        },
    }
}
