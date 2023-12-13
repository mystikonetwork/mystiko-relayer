use mystiko_protos::core::v1::SpendType;
use mystiko_relayer_types::TransactStatus;
use mystiko_storage::{DocumentData, IndexColumns};
use mystiko_storage_macros::CollectionBuilder;
use mystiko_types::{BridgeType, CircuitType};
use num_bigint::BigUint;

#[derive(CollectionBuilder, Clone, Debug, PartialEq)]
#[collection(indexes = indexes())]
pub struct Transaction {
    pub chain_id: u64,
    #[column(length_limit = 32)]
    pub spend_type: SpendType,
    #[column(length_limit = 64)]
    pub bridge_type: BridgeType,
    #[column(length_limit = 64)]
    pub status: TransactStatus,
    #[column(length_limit = 64)]
    pub pool_address: String,
    #[column(length_limit = 16)]
    pub asset_symbol: String,
    pub asset_decimals: u32,
    #[column(length_limit = 64)]
    pub circuit_type: CircuitType,
    pub proof: String,
    #[column(length_limit = 128)]
    pub root_hash: BigUint,
    pub output_commitments: Option<Vec<BigUint>>,
    #[column(length_limit = 255)]
    pub signature: String,
    pub serial_numbers: Option<Vec<BigUint>>,
    pub sig_hashes: Option<Vec<BigUint>>,
    #[column(length_limit = 255)]
    pub sig_pk: String,
    #[column(length_limit = 128)]
    pub public_amount: BigUint,
    #[column(length_limit = 128)]
    pub gas_relayer_fee_amount: BigUint,
    pub out_rollup_fees: Option<Vec<BigUint>>,
    #[column(length_limit = 64)]
    pub public_recipient: String,
    #[column(length_limit = 64)]
    pub relayer_recipient_address: String,
    pub out_encrypted_notes: Option<Vec<String>>,
    #[column(length_limit = 255)]
    pub random_auditing_public_key: BigUint,
    pub error_message: Option<String>,
    #[column(length_limit = 128)]
    pub transaction_hash: Option<String>,
}

fn indexes() -> Vec<IndexColumns> {
    vec![
        IndexColumns::builder()
            .column_names(vec![TransactionColumn::ChainId.to_string()])
            .build(),
        IndexColumns::builder()
            .column_names(vec![TransactionColumn::Signature.to_string()])
            .build(),
    ]
}
