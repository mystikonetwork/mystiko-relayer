use crate::configs::server::ServerConfig;
use async_trait::async_trait;
use mystiko_config::MystikoConfig;
use mystiko_ethers::{ChainProvidersOptions, ProviderOptions, ProvidersOptions, WS_REGEX};
use std::sync::Arc;
use typed_builder::TypedBuilder;

#[derive(Debug, TypedBuilder)]
#[builder(field_defaults(setter(into)))]
pub struct RelayerProviderOptions {
    relayer_config: Arc<ServerConfig>,
    mystiko_config: Arc<MystikoConfig>,
}

#[derive(Debug, TypedBuilder)]
#[builder(field_defaults(setter(into)))]
pub struct RelayerSignerOptions {
    relayer_config: Arc<ServerConfig>,
    mystiko_config: Arc<MystikoConfig>,
}

#[async_trait]
impl ChainProvidersOptions for RelayerProviderOptions {
    async fn providers_options(&self, chain_id: u64) -> anyhow::Result<Option<ProvidersOptions>> {
        todo!()
    }
}

#[async_trait]
impl ChainProvidersOptions for RelayerSignerOptions {
    async fn providers_options(&self, chain_id: u64) -> anyhow::Result<Option<ProvidersOptions>> {
        if let Some(chain_config) = self.mystiko_config.find_chain(chain_id) {
            // let signer_endpoint = self
            //     .relayer_config
            //     .chains
            //     .get(&chain_id)
            //     .map_or_else(|| chain_config.signer_endpoint().to_string(), |custom_chain_config| {
            //         custom_chain_config.signer_endpoint.clone().unwrap_or_else(|| chain_config.signer_endpoint().to_string())
            //     });

            let custom_signer_endpoint = self
                .relayer_config
                .chains
                .get(&chain_id)
                .and_then(|custom_chain_config| custom_chain_config.signer_endpoint.clone());

            let signer_endpoint = if let Some(signer_endpoint) = custom_signer_endpoint {
                signer_endpoint
            } else {
                chain_config.signer_endpoint().to_string()
            };

            let options = ProviderOptions::builder().url(signer_endpoint.to_string()).build();
            if WS_REGEX.is_match(&signer_endpoint) {
                Ok(Some(ProvidersOptions::Ws(options)))
            } else {
                Ok(Some(ProvidersOptions::Http(options)))
            }
        } else {
            Ok(None)
        }
    }
}
