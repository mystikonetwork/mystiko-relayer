use mystiko_storage::{DocumentData, UniqueColumns};
use mystiko_storage_macros::CollectionBuilder;

#[derive(CollectionBuilder, Clone, Debug, PartialEq)]
#[collection(uniques = uniques())]
pub struct Account {
    #[column(length_limit = 255)]
    pub chain_address: String,
    pub chain_id: u64,
    pub available: bool,
    pub supported_erc20_tokens: Vec<String>,
    pub balance_alarm_threshold: f64,
    pub balance_check_interval_ms: u64,
    pub insufficient_balances: bool,
}

fn uniques() -> Vec<UniqueColumns> {
    vec![vec![AccountColumn::ChainAddress, AccountColumn::ChainId].into()]
}
