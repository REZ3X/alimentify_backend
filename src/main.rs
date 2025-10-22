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

    let state = AppState {
        db,
        redis,
        config: config.clone(),
        gemini_service,
    };

    let app = routes
        ::create_routes(state.clone())
        .layer(middleware::cors::setup_cors(&config))
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([0, 0, 0, 0], config.server.port));
    tracing::info!("Alimentify API server starting on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.expect("Failed to bind to address");

    axum::serve(listener, app).await.expect("Failed to start server");
}
