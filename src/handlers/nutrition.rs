use axum::{ extract::State, http::StatusCode, response::IntoResponse, Json };
use axum_extra::extract::Multipart;
use serde::{ Deserialize, Serialize };
use std::sync::Arc;

use crate::{ db::AppState, error::AppError };

#[derive(Debug, Serialize)]
pub struct NutritionAnalysisResponse {
    pub success: bool,
    pub analysis: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct QuickCheckResponse {
    pub success: bool,
    pub quick_check: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub success: bool,
    pub error: String,
}

pub async fn analyze_food(
    State(state): State<AppState>,
    mut multipart: Multipart
) -> Result<impl IntoResponse, AppError> {
    tracing::info!("Received request for food nutrition analysis");

    let mut image_data: Option<Vec<u8>> = None;
    let mut mime_type: Option<String> = None;

    while
        let Some(field) = multipart
            .next_field().await
            .map_err(|e| AppError::BadRequest(format!("Failed to read multipart field: {}", e)))?
    {
        let field_name = field.name().unwrap_or("").to_string();

        if field_name == "image" {
            mime_type = field.content_type().map(|ct| ct.to_string());

            let data = field
                .bytes().await
                .map_err(|e| AppError::BadRequest(format!("Failed to read image data: {}", e)))?;

            image_data = Some(data.to_vec());
        }
    }

    let image_data = image_data.ok_or_else(|| {
        AppError::BadRequest("No image provided. Please upload an image file.".to_string())
    })?;

    if image_data.len() > 10 * 1024 * 1024 {
        return Err(AppError::BadRequest("Image too large. Maximum size is 10MB.".to_string()));
    }

    let mime_type = mime_type.unwrap_or_else(|| "image/jpeg".to_string());

    if !mime_type.starts_with("image/") {
        return Err(AppError::BadRequest("Invalid file type. Please upload an image.".to_string()));
    }

    tracing::info!("Processing image: {} bytes, mime_type: {}", image_data.len(), mime_type);

    let analysis = state.gemini_service
        .analyze_food_image(&image_data, &mime_type).await
        .map_err(|e| {
            tracing::error!("Gemini API error: {}", e);
            AppError::InternalError(e)
        })?;

    tracing::info!("Successfully analyzed food image");

    let response = NutritionAnalysisResponse {
        success: true,
        analysis,
        timestamp: chrono::Utc::now(),
    };

    Ok((StatusCode::OK, Json(response)))
}

pub async fn quick_food_check(
    State(state): State<AppState>,
    mut multipart: Multipart
) -> Result<impl IntoResponse, AppError> {
    tracing::info!("Received request for quick food check");

    let mut image_data: Option<Vec<u8>> = None;
    let mut mime_type: Option<String> = None;

    while
        let Some(field) = multipart
            .next_field().await
            .map_err(|e| AppError::BadRequest(format!("Failed to read multipart field: {}", e)))?
    {
        let field_name = field.name().unwrap_or("").to_string();

        if field_name == "image" {
            mime_type = field.content_type().map(|ct| ct.to_string());
            let data = field
                .bytes().await
                .map_err(|e| AppError::BadRequest(format!("Failed to read image data: {}", e)))?;
            image_data = Some(data.to_vec());
        }
    }

    let image_data = image_data.ok_or_else(|| {
        AppError::BadRequest("No image provided. Please upload an image file.".to_string())
    })?;

    if image_data.len() > 10 * 1024 * 1024 {
        return Err(AppError::BadRequest("Image too large. Maximum size is 10MB.".to_string()));
    }

    let mime_type = mime_type.unwrap_or_else(|| "image/jpeg".to_string());

    if !mime_type.starts_with("image/") {
        return Err(AppError::BadRequest("Invalid file type. Please upload an image.".to_string()));
    }

    tracing::info!("Processing quick check: {} bytes, mime_type: {}", image_data.len(), mime_type);

    let quick_check = state.gemini_service
        .quick_food_check(&image_data, &mime_type).await
        .map_err(|e| {
            tracing::error!("Gemini API error: {}", e);
            AppError::InternalError(e)
        })?;

    tracing::info!("Successfully completed quick food check");

    let response = QuickCheckResponse {
        success: true,
        quick_check,
        timestamp: chrono::Utc::now(),
    };

    Ok((StatusCode::OK, Json(response)))
}
