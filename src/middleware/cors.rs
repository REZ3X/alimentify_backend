use axum::http::{ header, Method, HeaderValue, HeaderName };
use tower_http::cors::CorsLayer;

use crate::config::Config;

pub fn setup_cors(config: &Config) -> CorsLayer {
    if !config.security.cors_enabled {
        return CorsLayer::permissive();
    }

    let allowed_origins: Vec<HeaderValue> = config.security.allowed_origins
        .iter()
        .filter_map(|origin| origin.parse().ok())
        .collect();

    CorsLayer::new()
        .allow_origin(allowed_origins)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::PATCH,
            Method::OPTIONS,
        ])
        .allow_headers([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            header::ACCEPT,
            HeaderName::from_static("x-api-key"),
        ])
        .allow_credentials(true)
        .max_age(std::time::Duration::from_secs(3600))
}
