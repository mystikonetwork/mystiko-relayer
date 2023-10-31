use mystiko_relayer_types::TransactStatus;
use mystiko_types::SpendType;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ChainStatusResponse {
    pub support: bool,
    pub available: bool,
    pub chain_id: u64,
    pub relayer_contract_address: Option<String>,
    pub contracts: Option<Vec<ContractResponse>>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ContractResponse {
    pub asset_symbol: String,
    pub relayer_fee_of_ten_thousandth: u32,
    pub minimum_gas_fee: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobStatusResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub job_type: SpendType,
    pub status: TransactStatus,
    pub response: Option<ResponseQueueData>,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseQueueData {
    pub hash: String,
    pub chain_id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TransactResponse {
    pub id: String,
    pub hash: String,
    pub chain_id: u64,
}
