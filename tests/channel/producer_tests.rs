use crate::channel::create_default_sender_and_receiver;
use crate::common::create_default_context;
use crate::handler::MockTransactions;
use actix_web::web::Data;
use mystiko_protos::core::v1::SpendType;
use mystiko_relayer::channel::producer::handler::TransactionProducer;
use mystiko_relayer::channel::producer::ProducerHandler;
use mystiko_relayer::channel::Channel;
use mystiko_relayer::database::transaction::Transaction;
use mystiko_relayer::service::find_sender;
use mystiko_relayer_types::TransactRequestData;
use mystiko_storage::Document;
use mystiko_types::{AssetType, CircuitType};
use serial_test::file_serial;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use typed_builder::TypedBuilder;

#[actix_rt::test]
#[file_serial]
async fn producer_send_success() {
    let context = create_default_context().await;
    let channel = Channel::new(Arc::new(context)).await.unwrap();
    let senders = Data::new(Arc::new(channel.senders));
    let sender = find_sender(senders, 5, "Mtt", AssetType::Erc20).unwrap();

    let result = sender
        .send(TransactRequestData {
            contract_param: Default::default(),
            spend_type: SpendType::Transfer,
            bridge_type: Default::default(),
            chain_id: 0,
            asset_symbol: "".to_string(),
            asset_decimals: 0,
            pool_address: "".to_string(),
            circuit_type: CircuitType::Rollup1,
            signature: "".to_string(),
        })
        .await;
    assert!(result.is_ok());
}

#[actix_rt::test]
async fn producer_send_success_v1() {
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
    let transaction_handler = MockTransactions::new();
    transaction_handler
        .expect_create_by_request()
        .withf(move |req| req.chain_id == data.chain_id)
        .returning(|res| {
            Ok(Document::new(
                String::from("123456"),
                1234567890u64,
                1234567891u64,
                Transaction {},
            ))
        });

    // create sender
    let (sender, _) = create_default_sender_and_receiver();

    let options = MockOptions::builder()
        .sender(sender)
        .handler(transaction_handler)
        .build();

    let producer = setup(options).await;
    let result = producer
        .send(TransactRequestData {
            contract_param: Default::default(),
            spend_type: SpendType::Transfer,
            bridge_type: Default::default(),
            chain_id: 0,
            asset_symbol: "".to_string(),
            asset_decimals: 0,
            pool_address: "".to_string(),
            circuit_type: CircuitType::Rollup1,
            signature: "".to_string(),
        })
        .await;
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
