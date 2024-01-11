use crate::channel::consumer::ConsumerHandler;
use crate::channel::Channel;
use crate::configs::server::ServerConfig;
use crate::context::Context;
use crate::database::{init_sqlite_database, Database};
use crate::service::handshake;
use crate::service::v1::handler::{chain_status, job_status, transact_v1};
use crate::service::v2::handler::{info, transact, transaction_status};
use actix_cors::Cors;
use actix_web::middleware::Logger;
use actix_web::web::{scope, Data};
use actix_web::{http, App, HttpServer};
use anyhow::Result;
use log::{info, LevelFilter};
use mystiko_storage::{SqlStatementFormatter, StatementFormatter, Storage};
use mystiko_storage_sqlite::SqliteStorage;
use std::str::FromStr;
use std::sync::Arc;

pub struct ApplicationOptions<F: StatementFormatter, S: Storage> {
    pub database: Arc<Database<F, S>>,
    pub context: Arc<Context>,
    pub channel: Channel,
}

impl<F, S> ApplicationOptions<F, S>
where
    F: StatementFormatter,
    S: Storage,
{
    pub async fn from_server_config(
        server_config: Arc<ServerConfig>,
    ) -> Result<ApplicationOptions<SqlStatementFormatter, SqliteStorage>> {
        // init sqlite db connection
        let database = Arc::new(init_sqlite_database(server_config.settings.sqlite_db_path.clone()).await?);
        // create context
        let context = Arc::new(Context::new(server_config.clone(), database.clone()).await?);
        // create channel
        let channel = Channel::new(context.clone()).await?;
        Ok(ApplicationOptions {
            database,
            context,
            channel,
        })
    }
}

pub async fn run_application<F, S>(options: ApplicationOptions<F, S>) -> Result<()>
where
    F: StatementFormatter,
    S: Storage,
{
    let server_config = options.context.server_config.clone();

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

    let channel = options.channel;
    let consumers = channel.consumers;
    let senders = Arc::new(channel.senders);

    // spawn consumer
    for mut consumer in consumers {
        tokio::spawn(async move {
            consumer.consume().await;
        });
    }

    // run http server
    let host = server_config.settings.host.as_str();
    let port = &server_config.settings.port;
    let api_version = &server_config.settings.api_version;

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
            .app_data(Data::new(options.context.clone()))
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
            )
    })
    .bind((host, *port))?
    .run()
    .await?;

    Ok(())
}
