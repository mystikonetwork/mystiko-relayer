use crate::common::TestServer;
use mystiko_relayer::channel::transact_channel;
use mystiko_relayer_types::TransactRequestData;
use mystiko_types::{AssetType, CircuitType, SpendType};
use serial_test::file_serial;

#[actix_rt::test]
#[file_serial]
async fn send_closed_channel() {
    let server = TestServer::new(None).await.unwrap();
    let app_state = server.app_state;
    let (senders, _) = transact_channel::init(
        &app_state.server_config,
        &app_state.relayer_config,
        &app_state.mystiko_config,
        server.providers.clone(),
        server.transaction_handler.clone(),
        server.token_price.clone(),
        1,
    )
    .await
    .unwrap();
    let producer = transact_channel::find_producer_by_id_and_symbol(&senders, 5, "Mtt", AssetType::Erc20);
    assert!(producer.is_some());
    let producer = producer.unwrap();

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
    assert!(result.is_err());
}
