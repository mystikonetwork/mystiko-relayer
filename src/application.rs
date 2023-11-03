use crate::channel::transact_channel;
use crate::common::init_app_state;
use crate::configs::{load_server_config, AccountConfig};
use crate::database::init_sqlite_database;
use crate::handler::account::AccountHandler;
use crate::handler::transaction::TransactionHandler;
use crate::service::{handshake, info, transact, transaction_status};
use crate::v1::service::{chain_status, job_status, transact_v1};
use actix_cors::Cors;
use actix_web::middleware::Logger;
use actix_web::web::{scope, Data};
use actix_web::{http, App, HttpServer};
use anyhow::Result;
use log::{info, LevelFilter};
use mystiko_ethers::{ChainConfigProvidersOptions, ProviderPool};
use mystiko_server_utils::token_price::config::TokenPriceConfig;
use mystiko_server_utils::token_price::TokenPrice;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;
use typed_builder::TypedBuilder;

#[derive(TypedBuilder)]
pub struct ApplicationOptions<'a> {
    array_queue_capacity: usize,
    server_config_path: Option<&'a str>,
}

#[allow(clippy::needless_lifetimes)]
pub async fn run_application<'a>(options: ApplicationOptions<'a>) -> Result<()> {
    // init server config
    let server_config = load_server_config(options.server_config_path)?;
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

    info!("load server config successful");

    let host = server_config.settings.host.as_str();
    let port = &server_config.settings.port;
    let api_version = &server_config.settings.api_version;
    let sqlite_db_path = &server_config.settings.sqlite_db_path;
    let accounts = &server_config.accounts;
    let network_type = &server_config.settings.network_type;
    let coin_market_cap_api_key = &server_config.settings.coin_market_cap_api_key;

    // init app state
    let app_state = init_app_state(server_config.clone()).await?;

    // init sqlite db connection
    let db = Arc::new(init_sqlite_database(sqlite_db_path.clone()).await?);

    // create account handler
    let account_handler = Arc::new(
        AccountHandler::new(
            db.clone(),
            accounts.values().cloned().collect::<Vec<AccountConfig>>().as_slice(),
        )
        .await?,
    );

    // create transaction handler
    let transaction_handler = Arc::new(TransactionHandler::new(db.clone()));

    // init token price
    let token_price = Arc::new(RwLock::new(TokenPrice::new(
        &TokenPriceConfig::new(serde_json::to_string(network_type)?.as_str(), None)?,
        coin_market_cap_api_key,
    )?));

    // init transact channel
    let providers: ProviderPool<ChainConfigProvidersOptions> = app_state.mystiko_config.clone().into();
    let providers = Arc::new(providers);

    let (senders, consumers) = transact_channel::init(
        &app_state.server_config,
        &app_state.relayer_config,
        providers.clone(),
        transaction_handler.clone(),
        token_price.clone(),
        options.array_queue_capacity,
    )
    .await?;

    // run transact consumers
    for consumer in consumers {
        tokio::spawn(async { consumer.run().await });
    }

    // run http server
    info!(
        "Application server start at {}:{}, available api version: {:?}",
        host, port, api_version
    );

    HttpServer::new(move || {
        // allow CORS request
        let cors = Cors::default()
            .allow_any_origin()
            .allowed_methods(vec!["GET", "POST"])
            .allowed_header(http::header::CONTENT_TYPE);
        // create app
        App::new()
            .wrap(cors)
            .wrap(Logger::default())
            .wrap(Logger::new("%a %{User-Agent}i"))
            .app_data(Data::new(app_state.clone()))
            .app_data(Data::new(senders.clone()))
            .app_data(Data::new(account_handler.clone()))
            .app_data(Data::new(transaction_handler.clone()))
            .app_data(Data::new(token_price.clone()))
            .app_data(Data::new(providers.clone()))
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
            )
    })
    .bind((host, *port))?
    .run()
    .await?;

    info!("Application server shutdown successful");

    Ok(())
}
