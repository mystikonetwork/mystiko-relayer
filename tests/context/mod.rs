use crate::common::{create_default_context, create_default_server_config};
use mockito::Server;
use mystiko_relayer::context::create_config;
use std::sync::Arc;

#[actix_rt::test]
async fn test_create_default_context() {
    let context = create_default_context().await;
    assert_eq!(context.server_config.settings.api_version.get(&0).unwrap(), "v2");
    assert_eq!(context.relayer_config.version(), "0.0.1");
    assert_eq!(context.mystiko_config.version(), "0.2.0");
    assert!(context.providers.get_provider(5).await.is_ok());
}

#[actix_rt::test]
async fn test_create_config_from_remote() {
    let mut server = Server::new_async().await;

    let mut server_config = create_default_server_config(true).await;
    server_config.options.mystiko_config_path = None;
    server_config.options.relayer_config_path = None;
    server_config.options.relayer_remote_config_base_url = Some(format!("{}/relayer_config", server.url()));
    server_config.options.mystiko_remote_config_base_url = Some(format!("{}/config", server.url()));

    // mock testnet
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
    let result = create_config(Arc::new(server_config)).await;
    assert!(result.is_ok());
    mock_0.assert_async().await;
    mock_1.assert_async().await;

    let mut server_config = create_default_server_config(false).await;
    server_config.options.mystiko_config_path = None;
    server_config.options.relayer_config_path = None;
    server_config.options.relayer_remote_config_base_url = Some(format!("{}/relayer_config", server.url()));
    server_config.options.mystiko_remote_config_base_url = Some(format!("{}/config", server.url()));

    // mock mainnet
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
    let result = create_config(Arc::new(server_config)).await;
    assert!(result.is_ok());
    mock_2.assert_async().await;
    mock_3.assert_async().await;
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
