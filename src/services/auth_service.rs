use chrono::{ Duration, Utc };
use jsonwebtoken::{ encode, EncodingKey, Header };
use oauth2::{
    basic::BasicClient,
    AuthUrl,
    ClientId,
    ClientSecret,
    RedirectUrl,
    TokenUrl,
    AuthorizationCode,
    TokenResponse,
};
use rand::Rng;
use redis::AsyncCommands;
use reqwest;

use crate::{
    config::Config,
    error::{ AppError, Result },
    models::{ Claims, GoogleUserInfo, Session, User },
};

const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const GOOGLE_USER_INFO_URL: &str = "https://www.googleapis.com/oauth2/v2/userinfo";

pub fn generate_google_auth_url(config: &Config) -> Result<String> {
    let client = BasicClient::new(
        ClientId::new(config.google_oauth.client_id.clone()),
        Some(ClientSecret::new(config.google_oauth.client_secret.clone())),
        AuthUrl::new(GOOGLE_AUTH_URL.to_string()).map_err(|e| AppError::InternalError(e.into()))?,
        Some(
            TokenUrl::new(GOOGLE_TOKEN_URL.to_string()).map_err(|e|
                AppError::InternalError(e.into())
            )?
        )
    ).set_redirect_uri(
        RedirectUrl::new(config.google_oauth.redirect_uri.clone()).map_err(|e|
            AppError::InternalError(e.into())
        )?
    );

    let (auth_url, _csrf_token) = client
        .authorize_url(oauth2::CsrfToken::new_random)
        .add_scope(oauth2::Scope::new("email".to_string()))
        .add_scope(oauth2::Scope::new("profile".to_string()))
        .url();

    Ok(auth_url.to_string())
}

pub async fn exchange_code_for_user(code: &str, config: &Config) -> Result<GoogleUserInfo> {
    let client = BasicClient::new(
        ClientId::new(config.google_oauth.client_id.clone()),
        Some(ClientSecret::new(config.google_oauth.client_secret.clone())),
        AuthUrl::new(GOOGLE_AUTH_URL.to_string()).map_err(|e| AppError::InternalError(e.into()))?,
        Some(
            TokenUrl::new(GOOGLE_TOKEN_URL.to_string()).map_err(|e|
                AppError::InternalError(e.into())
            )?
        )
    ).set_redirect_uri(
        RedirectUrl::new(config.google_oauth.redirect_uri.clone()).map_err(|e|
            AppError::InternalError(e.into())
        )?
    );

    let token_result = client
        .exchange_code(AuthorizationCode::new(code.to_string()))
        .request_async(oauth2::reqwest::async_http_client).await
        .map_err(|e| AppError::BadRequest(format!("Failed to exchange code: {}", e)))?;

    let access_token = token_result.access_token().secret();

    let http_client = reqwest::Client::new();
    let user_info: GoogleUserInfo = http_client
        .get(GOOGLE_USER_INFO_URL)
        .header("Authorization", format!("Bearer {}", access_token))
        .send().await
        .map_err(|e| AppError::InternalError(e.into()))?
        .json().await
        .map_err(|e| AppError::InternalError(e.into()))?;

    Ok(user_info)
}

pub fn generate_jwt_token(user: &User, config: &Config) -> Result<String> {
    let now = Utc::now().timestamp();
    let exp = now + config.jwt.expiration_hours * 3600;

    let claims = Claims {
        sub: user.id.as_ref().unwrap().to_hex(),
        email: user.gmail.clone(),
        exp,
        iat: now,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(config.jwt.secret.as_bytes())
    ).map_err(|e| AppError::InternalError(e.into()))
}

pub fn generate_verification_token() -> String {
    let mut rng = rand::thread_rng();
    let token: String = (0..32)
        .map(|_| {
            let idx = rng.gen_range(0..62);
            match idx {
                0..=25 => (b'A' + idx) as char,
                26..=51 => (b'a' + (idx - 26)) as char,
                _ => (b'0' + (idx - 52)) as char,
            }
        })
        .collect();
    token
}

pub async fn store_session(
    redis: &redis::aio::ConnectionManager,
    user: &User,
    _token: &str
) -> Result<()> {
    let mut conn = redis.clone();
    let user_id = user.id.as_ref().unwrap().to_hex();

    let ping_result: redis::RedisResult<String> = conn.get("test_ping").await;
    tracing::debug!("Redis ping result: {:?}", ping_result);

    let session = Session {
        user_id: user_id.clone(),
        email: user.gmail.clone(),
        created_at: Utc::now(),
        expires_at: Utc::now() + Duration::hours(24),
    };

    let session_json = serde_json::to_string(&session).map_err(|e| {
        tracing::error!("Failed to serialize session: {}", e);
        AppError::InternalError(e.into())
    })?;

    tracing::debug!("Storing session for user {}: {}", user_id, session_json);

    let key = format!("session:{}", user_id);

    conn.set_ex::<_, _, ()>(&key, session_json, 86400).await.map_err(|e| {
        tracing::error!("Failed to set session in Redis: {:?}", e);
        AppError::InternalError(anyhow::anyhow!("Redis error: {}", e))
    })?;

    tracing::info!("Successfully stored session for user {}", user_id);

    Ok(())
}

pub async fn delete_session(redis: &redis::aio::ConnectionManager, user_id: &str) -> Result<()> {
    let mut conn = redis.clone();
    let key = format!("session:{}", user_id);

    conn.del::<_, ()>(&key).await.map_err(|e| AppError::InternalError(e.into()))?;

    Ok(())
}
