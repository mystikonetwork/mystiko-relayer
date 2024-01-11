use anyhow::Result;
use async_trait::async_trait;
use ethers_core::types::{Bytes, U256};
use ethers_core::types::{TransactionReceipt, TxHash};
use mockall::mock;
use mystiko_abi::commitment_pool::TransactRequest;
use mystiko_ethers::{JsonRpcClientWrapper, Provider, ProviderWrapper};
use mystiko_protos::core::v1::SpendType;
use mystiko_relayer::configs::load_server_config;
use mystiko_relayer::configs::server::ServerConfig;
use mystiko_relayer::context::Context;
use mystiko_relayer::database::transaction::Transaction;
use mystiko_relayer::database::Database;
use mystiko_relayer_types::TransactRequestData;
use mystiko_server_utils::token_price::{PriceMiddleware, PriceMiddlewareError};
use mystiko_server_utils::tx_manager::TransactionMiddleware;
use mystiko_server_utils::tx_manager::{TransactionData, TransactionMiddlewareError};
use mystiko_storage::SqlStatementFormatter;
use mystiko_storage_sqlite::SqliteStorage;
use mystiko_types::{BridgeType, CircuitType};
use std::str::FromStr;
use std::sync::Arc;

#[allow(dead_code)]
const SERVER_CONFIG_TESTNET: &str = "tests/files/configs/config_testnet.toml";
#[allow(dead_code)]
const SERVER_CONFIG_MAINNET: &str = "tests/files/configs/config_mainnet.toml";
#[allow(dead_code)]
pub const RELAYER_CONFIG_PATH: &str = "tests/files/relayer_config.json";
#[allow(dead_code)]
pub const SERVER_CONFIG_INVALID_ID: &str = "tests/files/configs/config_invalid_id.toml";
#[allow(dead_code)]
pub const SERVER_CONFIG_INVALID_SYMBOL: &str = "tests/files/configs/config_invalid_symbol.toml";
#[allow(dead_code)]
pub const SERVER_CONFIG_INVALID_VERSION: &str = "tests/files/configs/config_invalid_version.toml";

#[allow(unused)]
pub async fn create_default_server_config(testnet: bool) -> ServerConfig {
    if testnet {
        load_server_config(Some(SERVER_CONFIG_TESTNET)).unwrap()
    } else {
        load_server_config(Some(SERVER_CONFIG_MAINNET)).unwrap()
    }
}

#[allow(unused)]
pub async fn create_default_database_in_memory() -> Database<SqlStatementFormatter, SqliteStorage> {
    let storage = SqliteStorage::from_memory().await.unwrap();
    let database = Database::new(SqlStatementFormatter::sqlite(), storage);
    database.migrate().await.unwrap();
    database
}

#[allow(unused)]
pub async fn create_default_context() -> Context {
    let server_config = create_default_server_config(true).await;
    let database = create_default_database_in_memory().await;
    Context::new(Arc::new(server_config), database).await.unwrap()
}
#[allow(unused)]
pub fn default_transaction() -> Transaction {
    Transaction {
        chain_id: 0,
        spend_type: Default::default(),
        bridge_type: Default::default(),
        status: Default::default(),
        pool_address: "".to_string(),
        asset_symbol: "".to_string(),
        asset_decimals: 0,
        circuit_type: CircuitType::Rollup1,
        proof: "".to_string(),
        root_hash: Default::default(),
        output_commitments: None,
        signature: "".to_string(),
        serial_numbers: None,
        sig_hashes: None,
        sig_pk: "".to_string(),
        public_amount: Default::default(),
        gas_relayer_fee_amount: Default::default(),
        out_rollup_fees: None,
        public_recipient: "".to_string(),
        relayer_recipient_address: "".to_string(),
        out_encrypted_notes: None,
        random_auditing_public_key: Default::default(),
        error_message: None,
        transaction_hash: None,
    }
}

#[allow(unused)]
pub fn default_transact_request_data(chain_id: u64) -> TransactRequestData {
    TransactRequestData {
        contract_param: TransactRequest {
            proof: Default::default(),
            root_hash: Default::default(),
            serial_numbers: vec![U256::from_str_radix(
                "0x19aaddbfd3840e5d9363793cc8a91c8e223db9775095316e528fe335db42956d",
                16,
            )
            .unwrap()],
            sig_hashes: vec![U256::from_str_radix(
                "0x0e5a093c5390514adad7e5277500319e7cc35d7682a4fa2ac84f4b5332909a5f",
                16,
            )
            .unwrap()],
            sig_pk: [
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 122, 235, 117, 149, 215, 12, 61, 182, 5, 183, 46, 6, 199, 169, 27,
                55, 117, 42, 27, 83,
            ],
            public_amount: U256::from_str_radix(
                "0x00000000000000000000000000000000000000000000000003fba0faba898000",
                16,
            )
            .unwrap(),
            relayer_fee_amount: U256::from_str_radix(
                "0x000000000000000000000000000000000000000000000000000aa87bee538000",
                16,
            )
            .unwrap(),
            out_commitments: vec![U256::from_str_radix(
                "0x1da10644733ab072dc6ea8aa6087d797b5002aa53238b753132448ba981102c5",
                16,
            )
            .unwrap()],
            out_rollup_fees: vec![U256::from_str_radix(
                "0x000000000000000000000000000000000000000000000000002386f26fc10000",
                16,
            )
            .unwrap()],
            public_recipient: Default::default(),
            relayer_address: Default::default(),
            out_encrypted_notes: vec![Bytes::from_str(
                "0x013b356d8d7b70e3896a4673b9a2c58904394a7d50bc92a6325b8\
                bedf6ec69ae938edaa562b23b50a7c62400ee344e6cedbb22233d53020d25e33650be5449b9ccd\
                94ca38c8ac66942c2d292b23149ec48b87de118acfab3895123e6eac243acf7a7055dbae309261\
                99852844ef19e2e43b065b697ae7a1faba33430240d380aa088ea5d207757780f412c401c503d7\
                3e3394703b6427a277f583a4bf368063966c32c4b3b238ebe0d60c544693d69c127529194da3bf\
                e5726064b96f7580802fa591dffea922139cfe2eccb6220d322a3",
            )
            .unwrap()],
            random_auditing_public_key: Default::default(),
            encrypted_auditor_notes: vec![],
        },
        spend_type: SpendType::Withdraw,
        bridge_type: BridgeType::Loop,
        chain_id,
        asset_symbol: "ETH".to_string(),
        asset_decimals: 18,
        pool_address: "0x4F416Acfd1153F9Af782056e68607227Af29D931".to_string(),
        circuit_type: CircuitType::Transaction1x0,
        signature: "0x800157ae47e94a156c42584190c33362b13ff94a7e8f5ef6ffd602c8d19ae\
        0684a4da6afd3c10bae9bd252dd20a9388d86c617bacb807a236a0285603e4086d61b"
            .to_string(),
    }
}

#[allow(unused)]
pub fn default_transaction_receipt(tx_hash: TxHash) -> TransactionReceipt {
    TransactionReceipt {
        transaction_hash: tx_hash,
        transaction_index: Default::default(),
        block_hash: None,
        block_number: None,
        from: Default::default(),
        to: None,
        cumulative_gas_used: Default::default(),
        gas_used: None,
        contract_address: None,
        logs: vec![],
        status: None,
        root: None,
        logs_bloom: Default::default(),
        transaction_type: None,
        effective_gas_price: None,
        other: Default::default(),
    }
}

mock! {
    #[derive(Debug)]
    pub TokenPrice {}

    #[async_trait]
    impl PriceMiddleware for TokenPrice {
        async fn price(&self, symbol: &str) -> Result<f64, PriceMiddlewareError>;
        async fn swap(
            &self,
            asset_a: &str,
            decimal_a: u32,
            amount_a: U256,
            asset_b: &str,
            decimal_b: u32,
        ) -> Result<U256, PriceMiddlewareError>;
    }
}

mock! {
    #[derive(Debug)]
    pub TxManager {}

    #[async_trait]
    impl TransactionMiddleware<ProviderWrapper<Box<dyn JsonRpcClientWrapper>>> for TxManager {
        fn tx_eip1559(&self) -> bool;
        async fn gas_price(&self, provider: &Provider) -> Result<U256, TransactionMiddlewareError>;
        async fn estimate_gas(&self, data: &TransactionData, provider: &Provider) -> Result<U256, TransactionMiddlewareError>;
        async fn send(&self, data: &TransactionData, provider: &Provider) -> Result<TxHash, TransactionMiddlewareError>;
        async fn confirm(
            &self,
            tx_hash: &TxHash,
            provider: &Provider,
        ) -> Result<TransactionReceipt, TransactionMiddlewareError>;
    }
}
