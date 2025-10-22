use axum::{
    extract::{ Request, State },
    http::StatusCode,
    middleware::Next,
    response::{ IntoResponse, Response },
    Json,
};
use jsonwebtoken::{ decode, DecodingKey, Validation };
use mongodb::bson::doc;
use serde_json::json;

use crate::{ db::AppState, models::{ Claims, User } };

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

    if state.config.security.require_email_verification {
        let users_collection = state.db.collection::<User>("users");
        let user_id = mongodb::bson::oid::ObjectId
            ::parse_str(&token_data.claims.sub)
            .map_err(|_| {
                (
                    StatusCode::UNAUTHORIZED,
                    Json(
                        json!({
                    "error": "Invalid user ID in token"
                })
                    ),
                ).into_response()
            })?;

        let user = users_collection
            .find_one(doc! { "_id": user_id }, None).await
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(
                        json!({
                        "error": "Failed to verify user"
                    })
                    ),
                ).into_response()
            })?
            .ok_or_else(|| {
                (
                    StatusCode::UNAUTHORIZED,
                    Json(
                        json!({
                        "error": "User not found"
                    })
                    ),
                ).into_response()
            })?;

        if !user.email_verification_status {
            return Err(
                (
                    StatusCode::FORBIDDEN,
                    Json(
                        json!({
                        "error": "Email verification required",
                        "message": "Please verify your email address before accessing this resource"
                    })
                    ),
                ).into_response()
            );
        }
    }

    request.extensions_mut().insert(token_data.claims);

    Ok(next.run(request).await)
}
