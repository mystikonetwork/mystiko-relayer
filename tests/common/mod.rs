use anyhow::Result;
use async_trait::async_trait;
use ethereum_types::U256;
use ethers_core::types::Bytes;
use ethers_middleware::providers::MockProvider;
use ethers_providers::Provider;
use lazy_static::lazy_static;
use log::LevelFilter;
use mockall::mock;
use mockito::{Server, ServerGuard};
use mystiko_abi::commitment_pool::TransactRequest;
use mystiko_ethers::{
    ChainConfigProvidersOptions, ChainProvidersOptions, ProviderPool, ProviderWrapper, Providers, ProvidersOptions,
};
use mystiko_protos::core::v1::SpendType;
use mystiko_relayer::channel::consumer::TransactionConsumer;
use mystiko_relayer::channel::{transact_channel, TransactSendersMap};
use mystiko_relayer::common::{init_app_state, AppState};
use mystiko_relayer::configs::{load_server_config, AccountConfig};
use mystiko_relayer::database::Database;
use mystiko_relayer::handler::account::AccountHandler;
use mystiko_relayer::handler::transaction::TransactionHandler;
use mystiko_relayer_types::TransactRequestData;
use mystiko_server_utils::token_price::config::TokenPriceConfig;
use mystiko_server_utils::token_price::TokenPrice;
use mystiko_storage::SqlStatementFormatter;
use mystiko_storage_sqlite::SqliteStorage;
use mystiko_types::{BridgeType, CircuitType};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

mock! {
    #[derive(Debug)]
    pub ChainConfig {}

    #[async_trait]
    impl ChainProvidersOptions for ChainConfig {
         async fn providers_options(&self, chain_id: u64) -> Result<Option<ProvidersOptions>>;
    }
}

#[allow(dead_code)]
pub const SERVER_CONFIG_SYMBOL_INVALID: &str = "./tests/files/config_symbol_invalid.toml";
#[allow(dead_code)]
pub const SERVER_CONFIG_VERSION_INVALID: &str = "./tests/files/config_version_invalid.toml";
#[allow(dead_code)]
pub const TOKEN_PRICE_CONFIG_PATH: &str = "./tests/files/token_price.json";
#[allow(dead_code)]
pub const TEST_RELAYER_CONFIG_SINGLE_PATH: &str = "./tests/files/relayer_config_single.json";
#[allow(dead_code)]
pub const SERVER_CONFIG_ID_NOT_FOUND: &str = "./tests/files/config_id_not_found.toml";
#[allow(dead_code)]
pub const TEST_RELAYER_CONFIG_PATH: &str = "./tests/files/relayer_config.json";
#[allow(dead_code)]
pub const TEST_MYSTIKO_CONFIG_PATH: &str = "./tests/files/mystiko_config.json";
pub const TESTNET_CONFIG_PATH: &str = "./tests/files/config_test_testnet.toml";
pub const MAINNET_CONFIG_PATH: &str = "./tests/files/config_test_mainnet.toml";
pub const ARRAY_QUEUE_CAPACITY: usize = 10;

pub struct TestServer {
    pub app_state: AppState,
    pub senders: TransactSendersMap,
    pub consumers: Vec<TransactionConsumer<ProviderPool<ChainConfigProvidersOptions>>>,
    pub account_handler: Arc<AccountHandler<SqlStatementFormatter, SqliteStorage>>,
    pub transaction_handler: Arc<TransactionHandler<SqlStatementFormatter, SqliteStorage>>,
    pub token_price: Arc<RwLock<TokenPrice>>,
    pub providers: Arc<ProviderPool<ChainConfigProvidersOptions>>,
    pub mock_server: ServerGuard,
}

impl TestServer {
    pub async fn new(mock: Option<MockProvider>) -> Result<Self> {
        // load server config
        let server_config = load_server_config(Some(TESTNET_CONFIG_PATH))?;

        let _ = env_logger::builder()
            .filter_module(
                "mystiko_relayer",
                LevelFilter::from_str(&server_config.settings.log_level)?,
            )
            .try_init();

        // init app state
        let app_state = init_app_state(server_config).await?;

        // create database in memory
        let storage = SqliteStorage::from_memory().await.unwrap();
        let database = Database::new(SqlStatementFormatter::sqlite(), storage);
        database.migrate().await.unwrap();
        let db = Arc::new(database);

        // handler
        let accounts: Vec<AccountConfig> = app_state.server_config.accounts.values().cloned().collect();
        let account_handler = Arc::new(AccountHandler::new(db.clone(), &accounts).await.unwrap());
        let transaction_handler = Arc::new(TransactionHandler::new(db.clone()));

        // init mock token price
        let server = Server::new_async().await;
        let mut default_cfg = TokenPriceConfig::new("testnet", None)?;
        default_cfg.base_url = server.url();
        let token_price = Arc::new(RwLock::new(TokenPrice::new(&default_cfg, "")?));

        // mock provider
        let mock = mock.unwrap_or_default();
        let provider: Arc<mystiko_ethers::Provider> = Arc::new(Provider::new(ProviderWrapper::new(Box::new(mock))));
        let pool: ProviderPool<ChainConfigProvidersOptions> = app_state.mystiko_config.clone().into();
        pool.delete_provider(5).await;
        pool.delete_provider(97).await;
        pool.set_provider(5, provider.clone()).await;
        pool.set_provider(97, provider.clone()).await;
        let providers = Arc::new(pool);

        // senders and consumers
        let (senders, consumers) = transact_channel::init(
            &app_state.server_config,
            &app_state.relayer_config,
            providers.clone(),
            transaction_handler.clone(),
            token_price.clone(),
            ARRAY_QUEUE_CAPACITY,
        )
        .await?;

        Ok(TestServer {
            app_state,
            senders,
            consumers,
            account_handler,
            transaction_handler,
            token_price,
            providers,
            mock_server: server,
        })
    }

    #[allow(dead_code)]
    pub async fn singleton() -> Arc<TestServer> {
        let mut server_opt = SERVER.lock().await;
        if server_opt.is_none() {
            *server_opt = Some(Arc::new(TestServer::new(None).await.unwrap()));
        }
        server_opt.clone().unwrap()
    }
}

lazy_static! {
    pub static ref SERVER: Mutex<Option<Arc<TestServer>>> = Mutex::new(None);
}

#[actix_rt::test]
async fn create_providers_chain_id_not_found() {
    let server = TestServer::new(None).await.unwrap();
    let app_state = server.app_state;
    let providers: ProviderPool<ChainConfigProvidersOptions> = app_state.mystiko_config.clone().into();
    let provider = providers.get_provider(999).await;
    assert!(provider.is_err());
}

#[actix_rt::test]
async fn init_app_state_from_remote() {
    let mut server = Server::new_async().await;

    let mut server_config = load_server_config(Some(TESTNET_CONFIG_PATH)).unwrap();
    server_config.options.mystiko_config_path = None;
    server_config.options.relayer_config_path = None;
    server_config.options.relayer_remote_config_base_url = Some(format!("{}/relayer_config", server.url()));
    server_config.options.mystiko_remote_config_base_url = Some(format!("{}/config", server.url()));

    // testnet
    let mock_0 = server
        .mock("GET", "/relayer_config/production/testnet/latest.json")
        .with_body(testnet_relayer_config_json_string())
        .create_async()
        .await;
    let mock_1 = server
        .mock("GET", "/config/production/testnet/latest.json")
        .with_body("{\"version\": \"0.2.0\"}")
        .create_async()
        .await;
    let app_state = init_app_state(server_config).await;
    println!("{:?}", app_state);
    assert!(app_state.is_ok());
    mock_0.assert_async().await;
    mock_1.assert_async().await;

    // mainnet
    let mut server_config = load_server_config(Some(MAINNET_CONFIG_PATH)).unwrap();
    server_config.options.mystiko_config_path = None;
    server_config.options.relayer_config_path = None;
    server_config.options.relayer_remote_config_base_url = Some(format!("{}/relayer_config", server.url()));
    server_config.options.mystiko_remote_config_base_url = Some(format!("{}/config", server.url()));

    let mock_2 = server
        .mock("GET", "/relayer_config/production/mainnet/latest.json")
        .with_body(mainnet_relayer_config_json_string())
        .create_async()
        .await;
    let mock_3 = server
        .mock("GET", "/config/production/mainnet/latest.json")
        .with_body("{\"version\": \"0.2.0\"}")
        .create_async()
        .await;
    let app_state = init_app_state(server_config).await;
    assert!(app_state.is_ok());
    mock_2.assert_async().await;
    mock_3.assert_async().await;
}

#[allow(dead_code)]
pub fn get_valid_transact_request_data() -> TransactRequestData {
    TransactRequestData {
        contract_param: TransactRequest {
            proof: Default::default(),
            root_hash: Default::default(),
            serial_numbers: vec![U256::from_str_radix(
                "0x19aaddbfd3840e5d9363793cc8a91c8e223db9775095316e528fe335db42956d",
                16,
            )
            .unwrap()],
            sig_hashes: vec![U256::from_str_radix(
                "0x0e5a093c5390514adad7e5277500319e7cc35d7682a4fa2ac84f4b5332909a5f",
                16,
            )
            .unwrap()],
            sig_pk: [
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 122, 235, 117, 149, 215, 12, 61, 182, 5, 183, 46, 6, 199, 169, 27,
                55, 117, 42, 27, 83,
            ],
            public_amount: U256::from_str_radix(
                "0x00000000000000000000000000000000000000000000000003fba0faba898000",
                16,
            )
            .unwrap(),
            relayer_fee_amount: U256::from_str_radix(
                "0x000000000000000000000000000000000000000000000000000aa87bee538000",
                16,
            )
            .unwrap(),
            out_commitments: vec![U256::from_str_radix(
                "0x1da10644733ab072dc6ea8aa6087d797b5002aa53238b753132448ba981102c5",
                16,
            )
            .unwrap()],
            out_rollup_fees: vec![U256::from_str_radix(
                "0x000000000000000000000000000000000000000000000000002386f26fc10000",
                16,
            )
            .unwrap()],
            public_recipient: Default::default(),
            relayer_address: Default::default(),
            out_encrypted_notes: vec![Bytes::from_str(
                "0x013b356d8d7b70e3896a4673b9a2c58904394a7d50bc92a6325b8\
                bedf6ec69ae938edaa562b23b50a7c62400ee344e6cedbb22233d53020d25e33650be5449b9ccd\
                94ca38c8ac66942c2d292b23149ec48b87de118acfab3895123e6eac243acf7a7055dbae309261\
                99852844ef19e2e43b065b697ae7a1faba33430240d380aa088ea5d207757780f412c401c503d7\
                3e3394703b6427a277f583a4bf368063966c32c4b3b238ebe0d60c544693d69c127529194da3bf\
                e5726064b96f7580802fa591dffea922139cfe2eccb6220d322a3",
            )
            .unwrap()],
            random_auditing_public_key: Default::default(),
            encrypted_auditor_notes: vec![],
        },
        spend_type: SpendType::Withdraw,
        bridge_type: BridgeType::Loop,
        chain_id: 5,
        asset_symbol: "ETH".to_string(),
        asset_decimals: 16,
        pool_address: "0x4F416Acfd1153F9Af782056e68607227Af29D931".to_string(),
        circuit_type: CircuitType::Transaction1x0,
        signature: "0x800157ae47e94a156c42584190c33362b13ff94a7e8f5ef6ffd602c8d19ae\
        0684a4da6afd3c10bae9bd252dd20a9388d86c617bacb807a236a0285603e4086d61b"
            .to_string(),
    }
}

fn testnet_relayer_config_json_string() -> String {
    let relayer_config = r#"
        {
          "chains":[
            {
              "assetDecimals":18,
              "assetSymbol":"ETH",
              "chainId":5,
              "contracts":[
                {
                  "assetDecimals":18,
                  "assetSymbol":"ETH",
                  "assetType":"main",
                  "relayerFeeOfTenThousandth":25
                },
                {
                  "assetDecimals":18,
                  "assetSymbol":"MTT",
                  "assetType":"erc20",
                  "relayerFeeOfTenThousandth":25
                },
                {
                  "assetDecimals":6,
                  "assetSymbol":"mUSD",
                  "assetType":"erc20",
                  "relayerFeeOfTenThousandth":25
                }
              ],
              "name":"Ethereum Goerli",
              "relayerContractAddress":"0x45B22A8CefDfF00989882CAE48Ad06D57938Efcc",
              "transactionInfo":{
                "erc20GasCost":{
                  "transaction1x0":512985,
                  "transaction1x1":629802,
                  "transaction1x2":705494,
                  "transaction2x0":611040,
                  "transaction2x1":727970,
                  "transaction2x2":803645
                },
                "mainGasCost":{
                  "transaction1x0":500704,
                  "transaction1x1":617592,
                  "transaction1x2":705128,
                  "transaction2x0":598799,
                  "transaction2x1":708389,
                  "transaction2x2":803183
                }
              }
            },
            {
              "assetDecimals":18,
              "assetSymbol":"BNB",
              "chainId":97,
              "contracts":[
                {
                  "assetDecimals":18,
                  "assetSymbol":"MTT",
                  "assetType":"erc20",
                  "relayerFeeOfTenThousandth":25
                },
                {
                  "assetDecimals":6,
                  "assetSymbol":"mUSD",
                  "assetType":"erc20",
                  "relayerFeeOfTenThousandth":25
                },
                {
                  "assetDecimals":18,
                  "assetSymbol":"BNB",
                  "assetType":"main",
                  "relayerFeeOfTenThousandth":25
                }
              ],
              "name":"BSC Testnet",
              "relayerContractAddress":"0xfC21Aa6a04f09565bC6eeDC182063Fd4E466670A",
              "transactionInfo":{
                "erc20GasCost":{
                  "transaction1x0":537145,
                  "transaction1x1":646754,
                  "transaction1x2":724302,
                  "transaction2x0":640808,
                  "transaction2x1":756699,
                  "transaction2x2":833563
                },
                "mainGasCost":{
                  "transaction1x0":520800,
                  "transaction1x1":636116,
                  "transaction1x2":724104,
                  "transaction2x0":630207,
                  "transaction2x1":743273,
                  "transaction2x2":833563
                }
              }
            }
          ],
          "gitRevision":"6335708",
          "version":"0.0.1"
        }
    "#;
    relayer_config.to_string()
}

fn mainnet_relayer_config_json_string() -> String {
    let relayer_config = r#"
        {
            "chains":[
                {
                    "assetDecimals":18,
                    "assetSymbol":"ETH",
                    "chainId":1,
                    "contracts":[
                        {
                            "assetDecimals":18,
                            "assetSymbol":"ETH",
                            "assetType":"main",
                            "relayerFeeOfTenThousandth":100
                        },
                        {
                            "assetDecimals":6,
                            "assetSymbol":"USDT",
                            "assetType":"erc20",
                            "relayerFeeOfTenThousandth":100
                        },
                        {
                            "assetDecimals":6,
                            "assetSymbol":"USDC",
                            "assetType":"erc20",
                            "relayerFeeOfTenThousandth":100
                        }
                    ],
                    "name":"Ethereum Mainnet",
                    "relayerContractAddress":"0xfeecaab7006A7f81acD36128c011395ab1D5FCe0",
                    "transactionInfo":{
                        "erc20GasCost":{
                            "transaction1x0":553636,
                            "transaction1x1":620019,
                            "transaction1x2":705494,
                            "transaction2x0":611040,
                            "transaction2x1":727970,
                            "transaction2x2":803645
                        },
                        "mainGasCost":{
                            "transaction1x0":500704,
                            "transaction1x1":619966,
                            "transaction1x2":705128,
                            "transaction2x0":598799,
                            "transaction2x1":708389,
                            "transaction2x2":803183
                        }
                    }
                }
            ],
            "gitRevision":"6335708",
            "version":"0.0.1"
        }
    "#;
    relayer_config.to_string()
}
