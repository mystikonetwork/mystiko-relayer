use async_trait::async_trait;
use mockall::mock;
use mystiko_ethers::Provider;
use std::sync::Arc;

mod consumer_tests;
mod producer_tests;

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
