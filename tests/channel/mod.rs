use async_trait::async_trait;
use ethers_providers::ProviderError;
use mockall::mock;
use mystiko_ethers::{JsonRpcClientWrapper, JsonRpcParams, Provider};
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

struct MockSenderAndReceiver {
    sender: Sender<(String, TransactRequestData)>,
    receiver: Receiver<(String, TransactRequestData)>,
}

#[warn(clippy::type_complexity)]
fn create_default_sender_and_receiver() -> MockSenderAndReceiver {
    let (sender, receiver) = channel::<(String, TransactRequestData)>(10);
    MockSenderAndReceiver { sender, receiver }
}
