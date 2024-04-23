use mystiko_protos::common::v1::ProviderType;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use typed_builder::TypedBuilder;
use validator::Validate;

#[derive(TypedBuilder, Validate, Serialize, Deserialize, Debug, Clone, Default)]
pub struct ChainConfig {
    pub provider_config: Option<ProviderConfig>,
    pub signer_endpoint: Option<String>,
}

#[derive(TypedBuilder, Validate, Serialize, Deserialize, Debug, Clone, Default)]
pub struct ProviderConfig {
    pub urls: HashMap<u16, String>,
    #[serde(default = "default_provider_type")]
    #[builder(default = default_provider_type())]
    pub provider_type: ProviderType,
}

fn default_provider_type() -> ProviderType {
    ProviderType::Failover
}
