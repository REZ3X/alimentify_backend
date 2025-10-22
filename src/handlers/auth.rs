use axum::{ extract::{ Query, State }, http::StatusCode, response::IntoResponse, Extension, Json };
use chrono::Utc;
use mongodb::bson::doc;
use serde::{ Deserialize, Serialize };
use serde_json::json;

use crate::{
    db::AppState,
    error::{ AppError, Result },
    models::{ AuthResponse, Claims, User, UserResponse },
    services::{ auth_service, email_service },
};

#[derive(Debug, Deserialize)]
pub struct GoogleCallbackQuery {
    pub code: String,
    #[allow(dead_code)]
    pub state: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AuthUrlResponse {
    pub auth_url: String,
}

pub async fn google_auth_url(State(state): State<AppState>) -> Result<Json<AuthUrlResponse>> {
    let auth_url = auth_service::generate_google_auth_url(&state.config)?;
    Ok(Json(AuthUrlResponse { auth_url }))
}

pub async fn google_callback(
    State(state): State<AppState>,
    Query(query): Query<GoogleCallbackQuery>
) -> Result<impl IntoResponse> {
    let google_user = auth_service::exchange_code_for_user(&query.code, &state.config).await?;

    let users_collection = state.db.collection::<User>("users");

    let mut user = match
        users_collection.find_one(doc! { "google_id": &google_user.id }, None).await
    {
        Ok(Some(user)) => {
            let mut updated_user = user.clone();
            updated_user.profile_image = google_user.picture.clone();
            updated_user.name = google_user.name.clone();
            updated_user.updated_at = Utc::now();

            users_collection
                .replace_one(doc! { "_id": user.id }, &updated_user, None).await
                .map_err(|e| AppError::InternalError(e.into()))?;

            updated_user
        }
        Ok(None) => {
            let username = google_user.email.split('@').next().unwrap_or("user").to_string();

            let verification_token = auth_service::generate_verification_token();

            let new_user = User {
                id: None,
                google_id: google_user.id.clone(),
                profile_image: google_user.picture.clone(),
                username,
                name: google_user.name.clone(),
                gmail: google_user.email.clone(),
                email_verification_status: false,
                email_verification_token: Some(verification_token.clone()),
                email_verified_at: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };

            let insert_result = users_collection
                .insert_one(&new_user, None).await
                .map_err(|e| AppError::InternalError(e.into()))?;

            if
                let Err(e) = email_service::send_verification_email(
                    &state.config,
                    &google_user.email,
                    &google_user.name,
                    &verification_token
                ).await
            {
                tracing::error!("Failed to send verification email: {}", e);
            }

            let mut created_user = new_user;
            created_user.id = Some(insert_result.inserted_id.as_object_id().unwrap());
            created_user
        }
        Err(e) => {
            return Err(AppError::InternalError(e.into()));
        }
    };

    let token = auth_service::generate_jwt_token(&user, &state.config)?;

    auth_service::store_session(&state.redis, &user, &token).await?;

    user.email_verification_token = None;

    let response = AuthResponse {
        token,
        user: user.into(),
    };

    Ok(Json(response))
}

pub async fn logout(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>
) -> Result<StatusCode> {
    auth_service::delete_session(&state.redis, &claims.sub).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_current_user(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>
) -> Result<Json<UserResponse>> {
    let users_collection = state.db.collection::<User>("users");

    let object_id = mongodb::bson::oid::ObjectId
        ::parse_str(&claims.sub)
        .map_err(|_| AppError::NotFound("Invalid user ID".to_string()))?;

    let user = users_collection
        .find_one(doc! { "_id": object_id }, None).await
        .map_err(|e| AppError::InternalError(e.into()))?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    Ok(Json(user.into()))
}

#[derive(Debug, Deserialize)]
pub struct VerifyEmailQuery {
    pub token: String,
}

pub async fn verify_email(
    State(state): State<AppState>,
    Query(query): Query<VerifyEmailQuery>
) -> Result<Json<serde_json::Value>> {
    let users_collection = state.db.collection::<User>("users");

    let user = users_collection
        .find_one(doc! { "email_verification_token": &query.token }, None).await
        .map_err(|e| AppError::InternalError(e.into()))?
        .ok_or_else(|| AppError::NotFound("Invalid verification token".to_string()))?;

    users_collection
        .update_one(
            doc! { "_id": user.id },
            doc! {
                "$set": {
                    "email_verification_status": true,
                    "email_verified_at": Utc::now(),
                    "email_verification_token": null,
                    "updated_at": Utc::now(),
                }
            },
            None
        ).await
        .map_err(|e| AppError::InternalError(e.into()))?;

    Ok(Json(json!({
        "message": "Email verified successfully"
    })))
}
