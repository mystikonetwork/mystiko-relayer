use crate::common::create_default_context;
use actix_web::web::Data;
use mystiko_protos::core::v1::SpendType;
use mystiko_relayer::channel::producer::ProducerHandler;
use mystiko_relayer::channel::Channel;
use mystiko_relayer::service::find_sender;
use mystiko_relayer_types::TransactRequestData;
use mystiko_types::{AssetType, CircuitType};
use serial_test::file_serial;
use std::sync::Arc;

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
