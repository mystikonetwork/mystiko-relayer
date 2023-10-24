use crate::handler::transaction::{TransactionHandler, UpdateTransactionOptions};
use anyhow::{bail, Result};
use ethers_core::abi::{AbiEncode, Address};
use ethers_core::types::{Bytes, TxHash, U256};
use log::{debug, error, info};
use mystiko_abi::commitment_pool::{CommitmentPool, TransactRequest};
use mystiko_ethers::{JsonRpcClientWrapper, Provider, ProviderWrapper, Providers};
use mystiko_relayer_types::{TransactRequestData, TransactStatus};
use mystiko_server_utils::token_price::PriceMiddleware;
use mystiko_server_utils::token_price::TokenPrice;
use mystiko_server_utils::tx_manager::TransactionData;
use mystiko_server_utils::tx_manager::{TransactionMiddleware, TxManager};
use mystiko_storage::SqlStatementFormatter;
use mystiko_storage_sqlite::SqliteStorage;
use std::ops::{Div, Mul};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::Receiver;
use tokio::sync::RwLock;
use tokio::time::sleep;

const MAX_GAS_PRICE_MULTIPLIER_LEGACY: u64 = 1;
const MAX_GAS_PRICE_MULTIPLIER_1559: u64 = 2;

pub struct TransactionConsumer<P: Providers = Box<dyn Providers>> {
    pub chain_id: u64,
    pub main_asset_symbol: String,
    pub main_asset_decimals: u32,
    pub receiver: Receiver<(String, TransactRequestData)>,
    pub providers: Arc<P>,
    pub signer: Arc<Provider>,
    pub handler: Arc<TransactionHandler<SqlStatementFormatter, SqliteStorage>>,
    pub token_price: Arc<RwLock<TokenPrice>>,
    pub tx_manager: TxManager<ProviderWrapper<Box<dyn JsonRpcClientWrapper>>>,
}

impl<P> TransactionConsumer<P>
where
    P: Providers,
{
    pub async fn run(mut self) {
        let chain_id = self.chain_id;
        info!("Launching a consumer for chain_id: {}", chain_id);
        loop {
            if let Some((id, data)) = self.receiver.recv().await {
                info!(
                    "consumer receive a transaction(id = {}, chain_id = {}, transaction_type = {:?})",
                    id, self.chain_id, data.transaction_type
                );

                let options = match self.consume(id.as_str(), &data).await {
                    Ok(tx_hash) => UpdateTransactionOptions::builder()
                        .status(TransactStatus::Succeeded)
                        .transaction_hash(tx_hash)
                        .build(),
                    Err(err) => {
                        error!("consume transaction error: {}", err);
                        UpdateTransactionOptions::builder()
                            .status(TransactStatus::Failed)
                            .error_message(err.to_string())
                            .build()
                    }
                };

                // update database
                self.update_transaction_status(id.as_str(), options).await;
            }
        }
    }

    async fn consume(&mut self, uuid: &str, data: &TransactRequestData) -> Result<String> {
        // get provider
        let provider = self.providers.get_provider(data.chain_id).await?;
        // get signer
        let signer = self.signer.clone();
        // parse address to Address
        let contract_address = Address::from_str(&data.pool_address)?;
        // build call data
        let call_data = self
            .build_call_data(contract_address, &provider, &data.contract_param, &data.signature)
            .await?;
        // get gas price
        let gas_price = self.tx_manager.gas_price(&provider).await?;
        // estimate gas
        let estimate_gas = self
            .estimate_gas(contract_address, &call_data, &signer, gas_price)
            .await?;
        // validate relayer fee
        let max_gas_price = self.validate_relayer_fee(data, &estimate_gas, gas_price).await?;
        // send transaction
        let tx_hash = self
            .send(contract_address, &call_data, &signer, estimate_gas, max_gas_price)
            .await?;

        // update transaction status to pending
        self.update_transaction_status(
            uuid,
            UpdateTransactionOptions::builder()
                .status(TransactStatus::Pending)
                .transaction_hash(tx_hash.clone())
                .build(),
        )
        .await;

        // wait transaction until confirmed
        info!(
            "Wait for the transaction(hash = {}, chain_id = {}) to be confirmed",
            tx_hash.as_str(),
            data.chain_id
        );
        self.wait_confirm(&signer, &tx_hash).await
    }

    async fn validate_relayer_fee(
        &mut self,
        data: &TransactRequestData,
        estimate_gas: &U256,
        gas_price: U256,
    ) -> Result<U256> {
        let out_rollup_fees = &data.contract_param.out_rollup_fees;
        let relayer_fee_amount = &data.contract_param.relayer_fee_amount;
        let asset_symbol = &data.asset_symbol;
        let asset_decimals = data.asset_decimals;

        debug!("out rollup fees = {:?}", out_rollup_fees);
        debug!("relayer fee amount = {:?}", relayer_fee_amount);
        debug!(
            "chain_id = {}, circuit_type = {:?}, estimate_gas = {}",
            self.chain_id, &data.circuit_type, estimate_gas
        );
        debug!("chain id = {}, gas price = {}", self.chain_id, gas_price);

        let estimate_transaction_fee_amount = gas_price.mul(estimate_gas);
        debug!("estimate transaction fee amount = {}", estimate_transaction_fee_amount);

        // swap estimate gas to asset symbol
        let price_service = self.token_price.write().await;
        // swap relayer fee to main asset symbol
        debug!(
            "relayer asset symbol = {}, decimals = {} swap to main asset symbol = {} decimals = {}",
            asset_symbol, asset_decimals, self.main_asset_symbol, self.main_asset_decimals
        );
        let relayer_fee_amount_main = price_service
            .swap(
                asset_symbol,
                asset_decimals,
                *relayer_fee_amount,
                self.main_asset_symbol.as_str(),
                self.main_asset_decimals,
            )
            .await?;
        drop(price_service);
        debug!(
            "swap relayer asset symbol = {} amount = {} to main symbol = {} amount = {}",
            asset_symbol, relayer_fee_amount, self.main_asset_symbol, relayer_fee_amount_main
        );

        // relayer_fee_amount_main > estimate_transaction_fee
        if relayer_fee_amount_main.lt(&estimate_transaction_fee_amount) {
            bail!(
                "Relayer fee amount not enough(relayer_fee_amount_main(symbol = {},decimals = {},amount = {}) \
                less than estimate_transaction_fee_amount(symbol = {},decimals = {},amount = {})",
                self.main_asset_symbol,
                self.main_asset_decimals,
                relayer_fee_amount_main,
                self.main_asset_symbol,
                self.main_asset_decimals,
                estimate_transaction_fee_amount,
            );
        }

        // max gas price_ref = relayer_fee_amount_main / estimate_gas
        let max_gas_price_ref = relayer_fee_amount_main.div(estimate_gas);
        let max_gas_price_multiplier = if self.tx_manager.support_1559() {
            MAX_GAS_PRICE_MULTIPLIER_1559
        } else {
            MAX_GAS_PRICE_MULTIPLIER_LEGACY
        };
        let max_gas_price = if max_gas_price_ref.gt(&gas_price.mul(max_gas_price_multiplier)) {
            gas_price.mul(max_gas_price_multiplier)
        } else {
            max_gas_price_ref
        };
        debug!(
            "relayer_fee_amount(symbol = {}, amount = {}), estimate_gas = {}, calculate max gas price = {}",
            self.main_asset_symbol, relayer_fee_amount_main, estimate_gas, max_gas_price,
        );

        info!(
            "validate relayer fee amount successful: relayer_fee_amount = {}\
            (asset_symbol = {}, asset_decimals = {}), max gas price reference value = {}",
            relayer_fee_amount, asset_symbol, asset_decimals, max_gas_price,
        );

        Ok(max_gas_price)
    }

    async fn send(
        &mut self,
        contract_address: Address,
        call_data: &Bytes,
        provider: &Arc<Provider>,
        gas_limit: U256,
        max_gas_price: U256,
    ) -> Result<String> {
        let data = TransactionData::builder()
            .to(contract_address)
            .data(call_data.to_vec().into())
            .value(U256::zero())
            .gas(gas_limit)
            .max_price(max_gas_price)
            .build();
        let tx_hash = self.tx_manager.send(&data, provider).await?.encode_hex();

        Ok(tx_hash)
    }

    async fn wait_confirm(&self, provider: &Arc<Provider>, tx_hash: &str) -> Result<String> {
        let tx_hash = TxHash::from_str(tx_hash)?;
        let receipt = self.tx_manager.confirm(&tx_hash, provider).await?;
        Ok(receipt.transaction_hash.encode_hex())
    }

    async fn build_call_data(
        &self,
        contract_address: Address,
        provider: &Arc<Provider>,
        data: &TransactRequest,
        signature: &str,
    ) -> Result<Bytes> {
        let contract = CommitmentPool::new(contract_address, provider.clone());
        let call_data = contract.transact(data.clone(), Bytes::from_str(signature)?).calldata();
        match call_data {
            None => {
                bail!("Invalid call data")
            }
            Some(result) => Ok(result),
        }
    }

    async fn update_transaction_status(&self, uuid: &str, options: UpdateTransactionOptions) {
        let mut retry_count = 0;
        let max_retry_count = 5;
        loop {
            if let Err(err) = self.handler.update_by_id(uuid, &options).await {
                error!(
                    "Failed to update transaction(id = {}) to status {:?}: {}",
                    uuid, &options.status, err
                );

                if retry_count >= max_retry_count {
                    error!(
                        "Exceeded maximum retry count. Failed to update transaction(id = {}) to status {:?}",
                        uuid, &options.status
                    );
                    break;
                }

                retry_count += 1;
                let wait_duration = Duration::from_secs(2);
                sleep(wait_duration).await;
                continue;
            } else {
                info!(
                    "Successfully update transaction(id = {}) to status {:?}",
                    uuid, &options.status
                );
                break;
            }
        }
    }

    async fn estimate_gas(
        &mut self,
        contract_address: Address,
        call_data: &Bytes,
        provider: &Arc<Provider>,
        gas_price: U256,
    ) -> Result<U256> {
        debug!("estimate gas for contract_address: {:?}", contract_address);
        let data = TransactionData::builder()
            .to(contract_address)
            .data(call_data.to_vec().into())
            .value(U256::zero())
            .gas(gas_price)
            .max_price(U256::from(100_000_000_000u64))
            .build();
        let estimate_gas = self.tx_manager.estimate_gas(&data, provider).await?;
        debug!("estimate gas successful: {}", estimate_gas);
        Ok(estimate_gas)
    }
}
