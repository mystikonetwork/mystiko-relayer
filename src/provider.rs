use std::sync::Arc;
use async_trait::async_trait;
use mystiko_config::MystikoConfig;
use mystiko_ethers::{ChainProvidersOptions, ProvidersOptions};
use mystiko_relayer_config::wrapper::relayer::RelayerConfig;
use typed_builder::TypedBuilder;

#[derive(Debug, TypedBuilder)]
#[builder(field_defaults(setter(into)))]
pub struct RelayerProviderOptions {
    relayer_config: Arc<RelayerConfig>,
    mystiko_config: Arc<MystikoConfig>,
}

#[async_trait]
impl ChainProvidersOptions for RelayerProviderOptions {
    async fn providers_options(&self, chain_id: u64) -> anyhow::Result<Option<ProvidersOptions>> {
        if  {  }
    }
}