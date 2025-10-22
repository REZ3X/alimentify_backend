use axum::{
    extract::{ Request, State },
    http::StatusCode,
    middleware::Next,
    response::{ IntoResponse, Response },
    Json,
};
use jsonwebtoken::{ decode, DecodingKey, Validation };
use serde_json::json;

use crate::{ db::AppState, models::Claims };

pub async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next
) -> Result<Response, Response> {
    let token = request
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(
                    json!({
                    "error": "Missing or invalid authorization header"
                })
                ),
            ).into_response()
        })?;

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(state.config.jwt.secret.as_bytes()),
        &Validation::default()
    ).map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "error": "Invalid or expired token"
            })),
        ).into_response()
    })?;

    request.extensions_mut().insert(token_data.claims);

    Ok(next.run(request).await)
}
