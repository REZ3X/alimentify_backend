use axum::{
    extract::{ Query, State },
    http::StatusCode,
    response::{ IntoResponse, Redirect },
    Extension,
    Json,
};
use chrono::Utc;
use mongodb::bson::doc;
use serde::{ Deserialize, Serialize };
use serde_json::json;

use crate::{
    db::AppState,
    error::{ AppError, Result },
    models::{ Claims, User, UserResponse },
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
    tracing::info!("Google callback received with code");

    let google_user = auth_service
        ::exchange_code_for_user(&query.code, &state.config).await
        .map_err(|e| {
            tracing::error!("Failed to exchange code for user: {}", e);
            e
        })?;

    tracing::info!("Successfully exchanged code for user: {}", google_user.email);

    let users_collection = state.db.collection::<User>("users");

    tracing::info!("Searching for existing user with google_id: {}", google_user.id);

    let user = match
        users_collection.find_one(doc! { "google_id": &google_user.id }, None).await
    {
        Ok(Some(mut user)) => {
            tracing::info!("Existing user found: {}", user.gmail);

            let user_id = user.id.ok_or_else(|| {
                tracing::error!("User has no ID");
                AppError::InternalError(anyhow::anyhow!("User has no ID"))
            })?;

            tracing::info!("Updating user profile for user_id: {}", user_id);

            let update_doc = doc! {
                "$set": {
                    "profile_image": &google_user.picture,
                    "name": &google_user.name,
                    "updated_at": Utc::now(),
                }
            };

            users_collection
                .update_one(doc! { "_id": user_id }, update_doc, None).await
                .map_err(|e| {
                    tracing::error!("Failed to update user in database: {}", e);
                    AppError::InternalError(e.into())
                })?;

            tracing::info!("User updated successfully");

            user.profile_image = google_user.picture.clone();
            user.name = google_user.name.clone();
            user.updated_at = Utc::now();
            user.id = Some(user_id); 

            user
        }
        Ok(None) => {
            tracing::info!("No existing user found, creating new user");
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
                health_profile: None,
                has_completed_health_survey: Some(false),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };

            tracing::info!("Inserting new user into database");

            let insert_result = users_collection
                .insert_one(&new_user, None).await
                .map_err(|e| {
                    tracing::error!("Failed to insert user: {}", e);
                    AppError::InternalError(e.into())
                })?;

            let inserted_id = insert_result
                .inserted_id
                .as_object_id()
                .ok_or_else(|| {
                    tracing::error!("Failed to get inserted ID");
                    AppError::InternalError(anyhow::anyhow!("Failed to get inserted ID"))
                })?;

            tracing::info!("New user created with ID: {}", inserted_id);

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
            created_user.id = Some(inserted_id);
            created_user
        }
        Err(e) => {
            tracing::error!("Database error while finding user: {}", e);
            return Err(AppError::InternalError(e.into()));
        }
    };

    if user.id.is_none() {
        tracing::error!("User object has no ID after creation/fetch");
        return Err(AppError::InternalError(anyhow::anyhow!("User has no ID")));
    }

    tracing::info!("Generating JWT token for user: {}", user.gmail);

    let token = auth_service::generate_jwt_token(&user, &state.config).map_err(|e| {
        tracing::error!("Failed to generate JWT token: {}", e);
        e
    })?;

    tracing::info!("JWT token generated successfully");

    tracing::info!("Storing session in Redis");

    auth_service::store_session(&state.redis, &user, &token).await.map_err(|e| {
        tracing::error!("Failed to store session in Redis: {}", e);
        e
    })?;

    tracing::info!("Session stored successfully for user: {}", user.gmail);

    let frontend_url = if state.config.is_production() {
        state.config.security.allowed_origins
            .first()
            .cloned()
            .unwrap_or_else(|| "http://localhost:3000".to_string())
    } else {
        "http://localhost:3000".to_string()
    };

    let redirect_url = format!("{}/?token={}", frontend_url, token);

    tracing::info!("Redirecting user {} to {}", user.gmail, redirect_url);

    Ok(Redirect::to(&redirect_url))
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

pub async fn debug_config(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(
        json!({
        "google_client_id": state.config.google_oauth.client_id,
        "google_redirect_uri": state.config.google_oauth.redirect_uri,
        "environment": format!("{:?}", state.config.server.environment),
    })
    )
}
