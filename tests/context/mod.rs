use crate::common::create_default_context;

#[actix_rt::test]
async fn test_create_default_context() {
    let context = create_default_context().await;
    assert_eq!(context.server_config.settings.api_version.get(&0).unwrap(), "v2");
    assert_eq!(context.relayer_config.version(), "0.0.1");
    assert_eq!(context.mystiko_config.version(), "0.2.0");
    assert!(context.providers.get_provider(5).await.is_ok());
}
