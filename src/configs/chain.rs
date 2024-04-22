use serde::{Deserialize, Serialize};
use typed_builder::TypedBuilder;
use validator::Validate;

#[derive(TypedBuilder, Validate, Serialize, Deserialize, Debug, Clone, Default)]
pub struct ChainConfig {
    pub providers: Vec<String>,
    pub signer_endpoint: String,
}