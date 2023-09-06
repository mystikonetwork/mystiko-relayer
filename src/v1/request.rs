use ethereum_types::Address;
use ethers_core::types::U256;
use mystiko_types::{BridgeType, CircuitType};
use mystiko_validator::validate::is_ethereum_address;
use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};

#[derive(Validate, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ChainStatusRequest {
    #[validate(range(min = 1))]
    pub chain_id: u64,
    #[validate]
    pub options: Option<ChainStatusOptions>,
}

#[derive(Validate, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ChainStatusOptions {
    #[validate(length(min = 1))]
    pub asset_symbol: String,
    #[validate(range(min = 1))]
    pub asset_decimals: u64,
    pub circuit_type: CircuitType,
}

#[derive(Validate, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TransactRequestV1 {
    pub proof: ProofStruct,
    pub root_hash: U256,
    pub serial_numbers: Vec<U256>,
    pub sig_hashes: Vec<U256>,
    // to [u8; 32]
    pub sig_pk: String,
    pub public_amount: U256,
    pub out_commitments: Vec<U256>,
    pub out_rollup_fees: Vec<U256>,
    pub public_recipient: Address,
    // to Vec<Bytes>
    pub out_encrypted_notes: Vec<String>,
    // to U256
    pub random_auditing_public_key: String,
    pub encrypted_auditor_notes: Vec<String>,
    pub signature: String,
    #[serde(rename = "type")]
    pub transaction_type: TransactionTypeV1,
    #[validate(range(min = 1))]
    pub chain_id: u64,
    #[serde(rename = "mystikoContractAddress")]
    #[validate(custom = "is_ethereum_address")]
    pub pool_address: String,
    #[serde(rename = "symbol")]
    pub asset_symbol: String,
    pub bridge_type: BridgeType,
    #[validate(custom = "is_valid_circuit_type")]
    pub circuit_type: CircuitType,
    pub relayer_fee_amount: U256,
    pub relayer_address: Address,
}

#[derive(Validate, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ProofStruct {
    pub a: G1PointStruct,
    pub b: G2PointStruct,
    pub c: G1PointStruct,
}

#[derive(Validate, Serialize, Deserialize, Debug)]
pub struct G1PointStruct {
    #[serde(rename = "X")]
    pub x: U256,
    #[serde(rename = "Y")]
    pub y: U256,
}

#[derive(Validate, Serialize, Deserialize, Debug)]
pub struct G2PointStruct {
    #[serde(rename = "X")]
    pub x: [U256; 2],
    #[serde(rename = "Y")]
    pub y: [U256; 2],
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum TransactionTypeV1 {
    Transfer,
    Withdraw,
}

fn is_valid_circuit_type(value: &CircuitType) -> Result<(), ValidationError> {
    let allowed = [
        CircuitType::Transaction1x0,
        CircuitType::Transaction1x1,
        CircuitType::Transaction1x2,
        CircuitType::Transaction2x0,
        CircuitType::Transaction2x1,
        CircuitType::Transaction2x2,
    ];
    if allowed.contains(value) {
        Ok(())
    } else {
        Err(ValidationError::new("invalid circuit type"))
    }
}
