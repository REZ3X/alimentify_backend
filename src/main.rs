mod config;
mod db;
mod routes;
mod handlers;
mod models;
mod error;
mod middleware;
mod services;

use std::net::SocketAddr;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{ layer::SubscriberExt, util::SubscriberInitExt };

use config::Config;
use db::AppState;

#[tokio::main]
async fn main() {
    tracing_subscriber
        ::registry()
        .with(
            tracing_subscriber::EnvFilter
                ::try_from_default_env()
                .unwrap_or_else(|_|
                    "alimentify=debug,tower_http=debug,axum::rejection=trace".into()
                )
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env().expect("Failed to load configuration");

    tracing::info!("Environment: {:?}", config.server.environment);
    tracing::info!("CORS enabled: {}", config.security.cors_enabled);
    tracing::info!("API key enabled: {}", config.security.api_key_enabled);

    let db = db::setup_database(&config).await.expect("Failed to connect to MongoDB");

    let redis = db::setup_redis(&config).await.expect("Failed to connect to Redis");

    let gemini_api_key = std::env
        ::var("GEMINI_API_KEY")
        .expect("GEMINI_API_KEY must be set in environment variables");
    let gemini_service = std::sync::Arc::new(
        services::gemini_service::GeminiService::new(gemini_api_key)
    );
    tracing::info!("Initialized Gemini AI service");

    let fdc_api_key = std::env
        ::var("FOOD_CENTRAL_API_KEY")
        .expect("FOOD_CENTRAL_API_KEY must be set in environment variables");
    let fdc_service = std::sync::Arc::new(services::fdc_service::FdcService::new(fdc_api_key));
    tracing::info!("Initialized FDC (Food Data Central) service");

    let ninja_api_key = std::env
        ::var("NINJA_NUTRITION_API_KEY")
        .expect("NINJA_NUTRITION_API_KEY must be set in environment variables");
    let ninja_service = std::sync::Arc::new(
        services::ninja_service::NinjaService::new(ninja_api_key)
    );
    tracing::info!("Initialized Ninja Nutrition service");

    let mealdb_service = std::sync::Arc::new(services::mealdb_service::MealDbService::new());
    tracing::info!("Initialized MealDB service");

    let state = AppState {
        db,
        redis,
        config: config.clone(),
        gemini_service,
        fdc_service,
        ninja_service,
        mealdb_service,
    };

    let app = routes
        ::create_routes(state.clone())
        .layer(middleware::cors::setup_cors(&config))
        .layer(TraceLayer::new_for_http());

    let addr = format!("{}:{}", config.server.host, config.server.port);
    tracing::info!("Alimentify API server starting on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await.expect("Failed to bind to address");

    axum::serve(listener, app).await.expect("Failed to start server");
}
