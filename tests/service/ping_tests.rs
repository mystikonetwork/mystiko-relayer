use crate::common::TestServer;
use actix_web::test::{call_and_read_body_json, init_service, TestRequest};
use actix_web::web::Data;
use actix_web::App;
use mystiko_relayer::service::handshake;
use mystiko_relayer_types::response::{ApiResponse, ResponseCode};
use mystiko_relayer_types::HandshakeResponse;
use serial_test::file_serial;

#[actix_rt::test]
#[file_serial]
async fn test_ping() {
    let server = TestServer::new(None).await.unwrap();
    let app = init_service(
        App::new()
            .app_data(Data::new(server.app_state.clone()))
            .service(handshake),
    )
    .await;

    let req = TestRequest::get().uri("/handshake").to_request();
    let resp: ApiResponse<HandshakeResponse> = call_and_read_body_json(&app, req).await;
    let result = resp.data.unwrap();

    assert_eq!(resp.code, ResponseCode::Successful as i32);
    assert_eq!(result.api_version.len(), 1);
    assert_eq!(result.api_version.get(0).unwrap(), "v2");
}
