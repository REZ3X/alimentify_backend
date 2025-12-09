use axum::{
    extract::{ Request, State },
    http::{ HeaderMap, StatusCode },
    middleware::Next,
    response::{ IntoResponse, Response },
};
use serde_json::json;

use crate::db::AppState;

const PUBLIC_PATHS: &[&str] = &["/", "/docs", "/status"];

pub async fn api_key_middleware(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Request,
    next: Next
) -> Result<Response, Response> {
    if !state.config.security.api_key_enabled {
        return Ok(next.run(request).await);
    }

    let path = request.uri().path();
    if PUBLIC_PATHS.contains(&path) {
        return Ok(next.run(request).await);
    }

    let api_key = headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                axum::Json(json!({
                    "error": "Missing API key"
                })),
            ).into_response()
        })?;

    if !state.config.security.api_keys.contains(&api_key.to_string()) {
        return Err(
            (
                StatusCode::UNAUTHORIZED,
                axum::Json(json!({
                "error": "Invalid API key"
            })),
            ).into_response()
        );
    }

    Ok(next.run(request).await)
}
