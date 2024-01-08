use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use typed_builder::TypedBuilder;
use validator::Validate;

#[derive(TypedBuilder, Validate, Serialize, Deserialize, Debug, Clone, Default)]
#[builder(field_defaults(setter(into)))]
pub struct AccountConfig {
    #[builder(default)]
    #[validate(range(min = 1))]
    pub chain_id: u64,
    #[builder(default)]
    pub private_key: String,
    #[serde(default = "default_available")]
    #[builder(default = default_available())]
    pub available: bool,
    #[builder(default)]
    pub supported_erc20_tokens: HashMap<u16, String>,
    #[serde(default)]
    #[builder(default)]
    pub balance_alarm_threshold: f64,
    #[serde(default)]
    #[builder(default)]
    pub balance_check_interval_ms: u64,
}

fn default_available() -> bool {
    true
}
