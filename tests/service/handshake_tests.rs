use crate::common::TestServer;
use actix_web::test::{call_and_read_body_json, init_service, TestRequest};
use actix_web::web::Data;
use actix_web::App;
use mystiko_relayer::service::handshake;
use mystiko_relayer_types::response::{ApiResponse, ResponseCode};
use mystiko_relayer_types::HandshakeResponse;

#[actix_rt::test]
async fn test_handshake() {
    let server = TestServer::new(None).await.unwrap();
    let app = init_service(
        App::new()
            .app_data(Data::new(server.app_state.clone()))
            .service(handshake),
    )
    .await;
    let req = TestRequest::get().uri("/handshake").to_request();
    let resp: ApiResponse<HandshakeResponse> = call_and_read_body_json(&app, req).await;
    assert_eq!(resp.code, ResponseCode::Successful as i32);
    assert!(resp.data.is_some());
    assert_eq!(resp.data.unwrap().api_version.len(), 1);
}
