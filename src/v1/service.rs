use crate::channel::transact_channel::find_producer_by_id_and_symbol;
use crate::channel::TransactSendersMap;
use crate::common::AppState;
use crate::error::ResponseError;
use crate::handler::account::handler::Account;
use crate::handler::account::AccountHandler;
use crate::handler::transaction::{Transaction, TransactionHandler};
use crate::service::minimum_gas_fee;
use crate::v1::request::{ChainStatusRequest, TransactRequestV1, TransactionTypeV1};
use crate::v1::response::{
    ChainStatusResponse, ContractResponse, JobStatusResponse, ResponseQueueData, TransactResponse,
};
use actix_web::web::{Data, Json, Path};
use actix_web::{get, post, Responder};
use anyhow::Result;
use ethers_core::types::{Bytes, U256};
use ethers_middleware::providers::Middleware;
use log::{debug, error, info};
use mystiko_abi::commitment_pool::{G1Point, G2Point, Proof, TransactRequest};
use mystiko_ethers::{ChainConfigProvidersOptions, ProviderPool, Providers};
use mystiko_protos::core::v1::SpendType;
use mystiko_relayer_types::response::success;
use mystiko_relayer_types::{RegisterOptions, TransactRequestData, TransactStatus};
use mystiko_server_utils::token_price::TokenPrice;
use mystiko_storage::SqlStatementFormatter;
use mystiko_storage_sqlite::SqliteStorage;
use mystiko_types::AssetType;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use validator::Validate;

#[post("status")]
pub async fn chain_status(
    request: Json<ChainStatusRequest>,
    data: Data<AppState>,
    handler: Data<Arc<Account<SqlStatementFormatter, SqliteStorage>>>,
    token_price: Data<Arc<RwLock<TokenPrice>>>,
    providers: Data<Arc<ProviderPool<ChainConfigProvidersOptions>>>,
) -> actix_web::Result<impl Responder, ResponseError> {
    info!("api v1 version chain status");

    let relayer_config = &data.relayer_config;
    let chain_id = request.chain_id;

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
                let gas_price = gas_price_by_chain_id(chain_id, providers.clone()).await;
                if gas_price.is_err() {
                    return Err(ResponseError::GetGasPriceError { chain_id });
                }
                let gas_price = gas_price.unwrap();
                debug!("chain id {} gas prices {:?}", chain_id, gas_price);

                let minimum_gas_fee = minimum_gas_fee(
                    &data.relayer_config,
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
    handler: Data<Arc<Transaction<SqlStatementFormatter, SqliteStorage>>>,
) -> actix_web::Result<impl Responder, ResponseError> {
    info!("api v1 version job status");

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
    data: Data<AppState>,
    senders: Data<TransactSendersMap>,
    handler: Data<Arc<Transaction<SqlStatementFormatter, SqliteStorage>>>,
) -> actix_web::Result<impl Responder, ResponseError> {
    info!("api v1 version transact");

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

    let pool_contract = data
        .mystiko_config
        .find_pool_contract_by_address(request.chain_id, request.pool_address.as_str());
    if pool_contract.is_none() {
        return Err(ResponseError::Unknown);
    }
    let pool_contract = pool_contract.unwrap();

    let request = parse_transact_request(request.into_inner(), pool_contract.asset_decimals()).map_err(|err| {
        error!("parse transact request error {:?}", err);
        ResponseError::Unknown
    })?;

    // save data and sent
    match find_producer_by_id_and_symbol(&senders, request.chain_id, &request.asset_symbol, asset_type) {
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

pub fn parse_transact_request(request: TransactRequestV1, asset_decimals: u32) -> Result<TransactRequestData> {
    let sig_pk = convert_sig_pk(request.sig_pk)?;
    let out_encrypted_notes = convert_out_encrypted_notes(request.out_encrypted_notes)?;
    let random_auditing_public_key = convert_random_auditing_public_key(request.random_auditing_public_key.as_str())?;
    let encrypted_auditor_notes = convert_encrypted_auditor_notes(request.encrypted_auditor_notes)?;
    Ok(TransactRequestData {
        contract_param: TransactRequest {
            proof: Proof {
                a: G1Point {
                    x: request.proof.a.x,
                    y: request.proof.a.y,
                },
                b: G2Point {
                    x: request.proof.b.x,
                    y: request.proof.b.y,
                },
                c: G1Point {
                    x: request.proof.c.x,
                    y: request.proof.c.y,
                },
            },
            root_hash: request.root_hash,
            serial_numbers: request.serial_numbers,
            sig_hashes: request.sig_hashes,
            sig_pk,
            public_amount: request.public_amount,
            relayer_fee_amount: request.relayer_fee_amount,
            out_commitments: request.out_commitments,
            out_rollup_fees: request.out_rollup_fees,
            public_recipient: request.public_recipient,
            relayer_address: request.relayer_address,
            out_encrypted_notes,
            random_auditing_public_key,
            encrypted_auditor_notes,
        },
        spend_type: convert_spend_type(request.transaction_type),
        bridge_type: request.bridge_type,
        chain_id: request.chain_id,
        asset_symbol: request.asset_symbol,
        asset_decimals,
        pool_address: request.pool_address,
        circuit_type: request.circuit_type,
        signature: request.signature,
    })
}

async fn gas_price_by_chain_id<P: Providers>(chain_id: u64, providers: Data<Arc<P>>) -> Result<U256> {
    let provider = providers.get_provider(chain_id).await?;

    let gas_price = provider.get_gas_price().await?;
    Ok(gas_price)
}

fn convert_sig_pk(sig_pk: String) -> Result<[u8; 32]> {
    let decode = hex::decode(&sig_pk[2..])?;
    let mut result = [0u8; 32];
    result.copy_from_slice(decode.as_slice());
    Ok(result)
}

fn convert_out_encrypted_notes(out_encrypted_notes: Vec<String>) -> Result<Vec<Bytes>> {
    let mut result: Vec<Bytes> = vec![];
    for notes in out_encrypted_notes {
        let decode = hex::decode(&notes[2..])?;
        let bytes: Bytes = Bytes::from(decode);
        result.push(bytes);
    }
    Ok(result)
}

fn convert_encrypted_auditor_notes(out_encrypted_notes: Vec<String>) -> Result<Vec<U256>> {
    let mut result: Vec<U256> = vec![];
    for notes in &out_encrypted_notes {
        result.push(U256::from_dec_str(notes)?);
    }
    debug!("convert encrypted auditor notes {:?}", result);
    Ok(result)
}

fn convert_random_auditing_public_key(key: &str) -> Result<U256> {
    let result = U256::from_dec_str(key)?;
    debug!("convert random auditing public key {:?}", result);
    Ok(result)
}

fn convert_spend_type(t: TransactionTypeV1) -> SpendType {
    match t {
        TransactionTypeV1::Transfer => SpendType::Transfer,
        TransactionTypeV1::Withdraw => SpendType::Withdraw,
    }
}
