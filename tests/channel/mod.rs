use crate::common::create_default_context;
use async_trait::async_trait;
use ethers_providers::ProviderError;
use mockall::mock;
use mystiko_ethers::{JsonRpcClientWrapper, JsonRpcParams, Provider};
use mystiko_relayer::channel::Channel;
use mystiko_relayer_types::TransactRequestData;
use std::sync::Arc;
use tokio::sync::mpsc::{channel, Receiver, Sender};

mod consumer_tests;
mod producer_tests;

mock! {
    #[derive(Debug)]
    pub Provider {}

    #[async_trait]
    impl JsonRpcClientWrapper for Provider {
         async fn request(
            &self,
            method: &str,
            params: JsonRpcParams,
        ) -> Result<serde_json::Value, ProviderError>;
    }
}

mock! {
    #[derive(Debug)]
    pub Providers {}

    #[async_trait]
    impl mystiko_ethers::Providers for Providers {
        async fn get_provider(&self, chain_id: u64) -> anyhow::Result<Arc<Provider>>;
        async fn has_provider(&self, chain_id: u64) -> bool;
        async fn set_provider(&self, chain_id: u64, provider: Arc<Provider>) -> Option<Arc<Provider>>;
        async fn delete_provider(&self, chain_id: u64) -> Option<Arc<Provider>>;
    }
}

fn create_default_sender_and_receiver() -> (
    Sender<(String, TransactRequestData)>,
    Receiver<(String, TransactRequestData)>,
) {
    channel::<(String, TransactRequestData)>(10)
}
