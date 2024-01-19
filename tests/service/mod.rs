use crate::channel::{MockConsumers, MockProducers};
use crate::common::{MockProviders, MockTokenPrice, SERVER_CONFIG_TESTNET};
use crate::handler::{MockAccounts, MockTransactions};
use actix_http::body::BoxBody;
use actix_http::Request;
use actix_web::dev::{Service, ServiceResponse};
use actix_web::test::{call_and_read_body_json, init_service, TestRequest};
use actix_web::web::{scope, Data};
use actix_web::{App, Error};
use anyhow::Result;
use async_trait::async_trait;
use ethers_core::types::{Block, FeeHistory, TxHash, U256};
use ethers_providers::ProviderError;
use log::LevelFilter;
use mystiko_ethers::{JsonRpcClientWrapper, JsonRpcParams, Provider, ProviderWrapper};
use mystiko_relayer::channel::consumer::ConsumerHandler;
use mystiko_relayer::channel::producer::ProducerHandler;
use mystiko_relayer::channel::SenderInfo;
use mystiko_relayer::configs::load_server_config;
use mystiko_relayer::context::{create_config, Context};
use mystiko_relayer::error::RelayerServerError;
use mystiko_relayer::service::handshake;
use mystiko_relayer::service::v1::handler::{chain_status, job_status, transact_v1};
use mystiko_relayer::service::v2::handler::{info, transact, transaction_status};
use mystiko_relayer_types::response::{ApiResponse, ResponseCode};
use mystiko_relayer_types::HandshakeResponse;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;
use typed_builder::TypedBuilder;

mod v1;
mod v2;

const CHAIN_ID: u64 = 99;

struct MockOptions {
    chain_id: u64,
    providers: HashMap<u64, MockProvider>,
    transaction_handler: MockTransactions,
    account_handler: MockAccounts,
    token_price: MockTokenPrice,
    producer: MockProducers,
    consumer: MockConsumers,
}

async fn create_app(
    options: MockOptions,
) -> Result<impl Service<Request, Response = ServiceResponse<BoxBody>, Error = Error> + Sized> {
    let server_config = Arc::new(load_server_config(Some(SERVER_CONFIG_TESTNET)).unwrap());
    let (relayer_config, mystiko_config) = create_config(server_config.clone()).await.unwrap();

    // try init logger
    let _ = env_logger::builder()
        .filter_module(
            "mystiko_relayer",
            LevelFilter::from_str(&server_config.settings.log_level)?,
        )
        .filter_module(
            "mystiko_server_utils",
            LevelFilter::from_str(&server_config.settings.log_level)?,
        )
        .try_init();

    let mut providers = MockProviders::new();
    let mut raw_providers = options
        .providers
        .into_iter()
        .map(|(chain_id, provider)| {
            let provider = Arc::new(Provider::new(ProviderWrapper::new(Box::new(provider))));
            (chain_id, provider)
        })
        .collect::<HashMap<_, _>>();
    providers.expect_get_provider().returning(move |chain_id| {
        raw_providers
            .remove(&chain_id)
            .ok_or(anyhow::anyhow!("No provider for chain_id {}", chain_id))
    });

    let context = Context {
        server_config,
        relayer_config,
        mystiko_config,
        providers: Arc::new(Box::new(providers)),
        transaction_handler: Arc::new(Box::new(options.transaction_handler)),
        account_handler: Arc::new(Box::new(options.account_handler)),
        token_price: Arc::new(RwLock::new(Box::new(options.token_price))),
    };

    let mut senders = HashSet::new();
    senders.insert(SenderInfo {
        chain_id: options.chain_id,
        private_key: "0x000000".to_string(),
        supported_erc20_tokens: vec!["MTT".to_string(), "ETH".to_string()],
        producer: Arc::new(Box::new(options.producer) as Box<dyn ProducerHandler<Error = RelayerServerError>>),
    });

    let consumers = vec![Box::new(options.consumer) as Box<dyn ConsumerHandler>];
    let senders = Arc::new(senders);
    // spawn consumer
    for mut consumer in consumers {
        tokio::spawn(async move {
            consumer.consume().await;
        });
    }

    // run http server
    let app = init_service(
        App::new()
            .app_data(Data::new(Arc::new(context.clone())))
            .app_data(Data::new(senders.clone()))
            .service(handshake)
            // v1
            .service(chain_status)
            .service(job_status)
            .service(transact_v1)
            .service(
                scope("/api/v2")
                    .service(info)
                    .service(transact)
                    .service(transaction_status),
            ),
    )
    .await;

    Ok(app)
}

#[actix_rt::test]
async fn test_handshake() {
    let options = MockOptions {
        chain_id: CHAIN_ID,
        providers: HashMap::new(),
        transaction_handler: MockTransactions::new(),
        account_handler: MockAccounts::new(),
        token_price: MockTokenPrice::new(),
        consumer: MockConsumers::new(),
        producer: MockProducers::new(),
    };
    let app = create_app(options).await.unwrap();
    let request = TestRequest::get().uri("/handshake").to_request();
    let response: ApiResponse<HandshakeResponse> = call_and_read_body_json(&app, request).await;
    assert_eq!(response.code, ResponseCode::Successful as i32);
    assert!(response.data.is_some());
    assert_eq!(response.data.unwrap().api_version, vec!["v2".to_string()]);
}

#[derive(Debug, TypedBuilder)]
struct MockProvider {
    #[builder(default)]
    base_fee_per_gas: U256,
    #[builder(default)]
    max_fee_per_gas: U256,
    #[builder(default)]
    priority_fee: U256,
    #[builder(default)]
    gas_price: U256,
}

#[async_trait]
impl JsonRpcClientWrapper for MockProvider {
    async fn request(&self, method: &str, _params: JsonRpcParams) -> std::result::Result<Value, ProviderError> {
        if method == "eth_getBlockByNumber" {
            let block = Block::<TxHash> {
                base_fee_per_gas: Some(self.base_fee_per_gas),
                ..Default::default()
            };
            Ok(serde_json::json!(block))
        } else if method == "eth_estimateEip1559Fees" {
            Ok(serde_json::json!((self.max_fee_per_gas, self.priority_fee)))
        } else if method == "eth_feeHistory" {
            let history = FeeHistory {
                base_fee_per_gas: vec![],
                gas_used_ratio: vec![],
                oldest_block: Default::default(),
                reward: vec![],
            };
            Ok(serde_json::json!(history))
        } else if method == "eth_gasPrice" {
            Ok(serde_json::json!(self.gas_price))
        } else {
            panic!("Unexpected method: {}", method);
        }
    }
}
