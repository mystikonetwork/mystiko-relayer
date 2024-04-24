use crate::configs::server::ServerConfig;
use crate::error::RelayerServerError;
use async_trait::async_trait;
use ethers_providers::Quorum;
use mystiko_config::MystikoConfig;
use mystiko_ethers::{ChainProvidersOptions, ProviderOptions, ProvidersOptions, QuorumProviderOptions, WS_REGEX};
use mystiko_protos::common::v1::ProviderType;
use std::sync::Arc;
use std::time::Duration;
use typed_builder::TypedBuilder;

#[derive(Debug, TypedBuilder)]
#[builder(field_defaults(setter(into)))]
pub struct RelayerProviderOptions {
    server_config: Arc<ServerConfig>,
    mystiko_config: Arc<MystikoConfig>,
}

#[derive(Debug, TypedBuilder)]
#[builder(field_defaults(setter(into)))]
pub struct RelayerSignerOptions {
    server_config: Arc<ServerConfig>,
    mystiko_config: Arc<MystikoConfig>,
}

#[async_trait]
impl ChainProvidersOptions for RelayerProviderOptions {
    async fn providers_options(&self, chain_id: u64) -> anyhow::Result<Option<ProvidersOptions>> {
        if let Some(chain_config) = self.mystiko_config.find_chain(chain_id) {
            let mut providers_options: Vec<ProviderOptions> = vec![];
            let mut provider_type = ProviderType::Failover;

            if let Some(relayer_chain_config) = self.server_config.chains.get(&chain_id) {
                if let Some(provider_config) = &relayer_chain_config.provider_config {
                    provider_type = provider_config.provider_type;
                    for url in provider_config.urls.values() {
                        providers_options.push(ProviderOptions::builder().url(url.to_string()).build());
                    }
                }
            }

            if providers_options.is_empty() {
                for provider_config in chain_config.providers() {
                    let provider_options = ProviderOptions::builder()
                        .url(provider_config.url().to_string())
                        .quorum_weight(provider_config.quorum_weight() as u64)
                        .timeout_retries(provider_config.max_try_count() - 1)
                        .rate_limit_retries(provider_config.max_try_count() - 1)
                        .request_timeout(Duration::from_millis(provider_config.timeout_ms() as u64))
                        .build();
                    providers_options.push(provider_options);
                }
            }

            match provider_type {
                ProviderType::Unspecified => Err(RelayerServerError::ProviderTypeUnspecifiedError())?,
                ProviderType::Failover => Ok(Some(ProvidersOptions::Failover(providers_options))),
                ProviderType::Quorum => {
                    let quorum_options = QuorumProviderOptions::builder()
                        .quorum(Quorum::Percentage(chain_config.provider_quorum_percentage()))
                        .build();
                    Ok(Some(ProvidersOptions::Quorum(providers_options, quorum_options)))
                }
            }
        } else {
            Ok(None)
        }
    }
}

#[async_trait]
impl ChainProvidersOptions for RelayerSignerOptions {
    async fn providers_options(&self, chain_id: u64) -> anyhow::Result<Option<ProvidersOptions>> {
        if let Some(chain_config) = self.mystiko_config.find_chain(chain_id) {
            let signer_endpoint = if let Some(relayer_chain_config) = self.server_config.chains.get(&chain_id) {
                if let Some(signer_endpoint) = &relayer_chain_config.signer_endpoint {
                    signer_endpoint.to_string()
                } else {
                    chain_config.signer_endpoint().to_string()
                }
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
