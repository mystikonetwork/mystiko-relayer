use async_trait::async_trait;
use mockall::mock;
use mystiko_relayer::channel::consumer::ConsumerHandler;
use mystiko_relayer::channel::producer::ProducerHandler;
use mystiko_relayer::channel::SenderInfo;
use mystiko_relayer::database::transaction::Transaction;
use mystiko_relayer::error::RelayerServerError;
use mystiko_relayer_types::TransactRequestData;
use mystiko_storage::Document;
use std::sync::Arc;
use tokio::sync::mpsc::{channel, Receiver, Sender};

mod consumer_tests;
mod producer_tests;

struct MockSenderAndReceiver {
    sender: Sender<(String, TransactRequestData)>,
    receiver: Receiver<(String, TransactRequestData)>,
}

#[warn(clippy::type_complexity)]
fn create_default_sender_and_receiver() -> MockSenderAndReceiver {
    let (sender, receiver) = channel::<(String, TransactRequestData)>(10);
    MockSenderAndReceiver { sender, receiver }
}

#[test]
fn test_compare_sender_info() {
    let sender_0 = SenderInfo {
        chain_id: 1,
        private_key: "0x00000".to_string(),
        supported_erc20_tokens: vec![],
        producer: Arc::new(MockProducers::new()),
    };
    let sender_1 = SenderInfo {
        chain_id: 1,
        private_key: "0x00000".to_string(),
        supported_erc20_tokens: vec![],
        producer: Arc::new(MockProducers::new()),
    };
    let sender_2 = SenderInfo {
        chain_id: 2,
        private_key: "0x00000".to_string(),
        supported_erc20_tokens: vec![],
        producer: Arc::new(MockProducers::new()),
    };
    assert!(sender_0.eq(&sender_1));
    assert!(sender_0.ne(&sender_2));
}

mock! {
    pub Producers {}

    #[async_trait]
    impl ProducerHandler for Producers {
        type Error = RelayerServerError;
        async fn send(&self, data: TransactRequestData) -> Result<Document<Transaction>, RelayerServerError>;
    }
}

mock! {
    pub Consumers {}

    #[async_trait]
    impl ConsumerHandler for Consumers {
        async fn consume(&mut self);
    }
}
