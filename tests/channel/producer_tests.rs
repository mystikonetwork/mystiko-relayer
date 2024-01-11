use crate::channel::create_default_sender_and_receiver;
use crate::common::{default_transact_request_data, default_transaction};
use crate::handler::MockTransactions;
use mystiko_protos::core::v1::SpendType;
use mystiko_relayer::channel::producer::handler::TransactionProducer;
use mystiko_relayer::channel::producer::ProducerHandler;
use mystiko_relayer_types::TransactRequestData;
use mystiko_storage::Document;
use mystiko_types::CircuitType;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use typed_builder::TypedBuilder;

#[actix_rt::test]
async fn test_producer_send_success() {
    // data
    let data = TransactRequestData {
        contract_param: Default::default(),
        spend_type: SpendType::Transfer,
        bridge_type: Default::default(),
        chain_id: 0,
        asset_symbol: "".to_string(),
        asset_decimals: 0,
        pool_address: "".to_string(),
        circuit_type: CircuitType::Rollup1,
        signature: "".to_string(),
    };

    // mock handler
    let mut transaction_handler = MockTransactions::new();
    transaction_handler
        .expect_create_by_request()
        .withf(move |req| req.chain_id == data.chain_id)
        .returning(|_| {
            Ok(Document::new(
                String::from("123456"),
                1234567890u64,
                1234567891u64,
                default_transaction(),
            ))
        });
    transaction_handler
        .expect_update_by_id()
        .withf(move |id, _| id == "123456")
        .returning(|_, _| {
            Ok(Some(Document::new(
                String::from("123456"),
                1234567890u64,
                1234567891u64,
                default_transaction(),
            )))
        });

    // create sender
    let mock = create_default_sender_and_receiver();
    let sender = mock.sender;

    let options = MockOptions::builder()
        .sender(sender)
        .handler(transaction_handler)
        .build();

    let producer = setup(options).await;
    let result = producer.send(default_transact_request_data(0)).await;
    assert!(result.is_ok());
}

#[derive(Debug, TypedBuilder)]
struct MockOptions {
    sender: Sender<(String, TransactRequestData)>,
    handler: MockTransactions,
}

async fn setup(options: MockOptions) -> TransactionProducer {
    TransactionProducer::new(Arc::new(options.sender), Arc::new(Box::new(options.handler)))
}
