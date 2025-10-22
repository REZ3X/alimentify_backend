use axum::{ http::StatusCode, Json };
use serde_json::{ json, Value };

pub async fn status_check() -> (StatusCode, Json<Value>) {
    (
        StatusCode::OK,
        Json(
            json!({
            "status": "healthy",
            "service": "Alimentify API",
            "version": env!("CARGO_PKG_VERSION"),
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "environment": std::env::var("NODE_ENV").unwrap_or_else(|_| "development".to_string()),
        })
        ),
    )
}
