use mongodb::{ Client, Database, options::{ ClientOptions, ServerApi, ServerApiVersion } };
use redis::aio::ConnectionManager;
use anyhow::Result;
use std::sync::Arc;

use crate::config::Config;
use crate::services::gemini_service::GeminiService;
use crate::services::fdc_service::FdcService;
use crate::services::ninja_service::NinjaService;

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub redis: ConnectionManager,
    pub config: Config,
    pub gemini_service: Arc<GeminiService>,
    pub fdc_service: Arc<FdcService>,
    pub ninja_service: Arc<NinjaService>,
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
    tracing::info!("Attempting to connect to Redis...");

    let client = redis::Client
        ::open(config.redis.url.as_str())
        .map_err(|e| anyhow::anyhow!("Failed to create Redis client: {}", e))?;

    let connection = ConnectionManager::new(client).await.map_err(|e|
        anyhow::anyhow!(
            "Failed to connect to Redis: {}. Please check your REDIS_URL and network connectivity.",
            e
        )
    )?;

    tracing::info!("Successfully connected to Redis");

    Ok(connection)
}
