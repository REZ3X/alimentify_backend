use mongodb::{ Client, Database, options::{ ClientOptions, ServerApi, ServerApiVersion } };
use redis::aio::ConnectionManager;
use anyhow::Result;

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub redis: ConnectionManager,
    pub config: Config,
}

pub async fn setup_database(config: &Config) -> Result<Database> {
    let mut client_options = ClientOptions::parse(&config.mongodb.uri).await?;

    let server_api = ServerApi::builder().version(ServerApiVersion::V1).build();
    client_options.server_api = Some(server_api);

    let client = Client::with_options(client_options)?;
    let database = client.database(&config.mongodb.database_name);

    database.list_collection_names(None).await?;

    tracing::info!("Connected to MongoDB: {}", config.mongodb.database_name);

    Ok(database)
}

pub async fn setup_redis(config: &Config) -> Result<ConnectionManager> {
    let client = redis::Client::open(config.redis.url.as_str())?;
    let connection = ConnectionManager::new(client).await?;

    tracing::info!("Connected to Redis");

    Ok(connection)
}
