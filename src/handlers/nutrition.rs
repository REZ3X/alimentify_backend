use axum::{ extract::State, http::StatusCode, response::IntoResponse, Json };
use axum_extra::extract::Multipart;
use serde::{ Deserialize, Serialize };

use crate::{ db::AppState, error::AppError };

#[derive(Debug, Serialize)]
pub struct NutritionAnalysisResponse {
    pub success: bool,
    pub analysis: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_valid_food: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct FoodTextRequest {
    pub food_description: String,
}

#[derive(Debug, Serialize)]
pub struct FoodNutritionDetails {
    pub food_name: String,
    pub calories: f64,
    pub protein_g: f64,
    pub carbs_g: f64,
    pub fat_g: f64,
    pub serving_size: String,
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
    let mut field_count = 0;

    while
        let Some(field) = multipart.next_field().await.map_err(|e| {
            tracing::error!("Multipart parsing error: {}", e);
            AppError::BadRequest(
                format!("Failed to read multipart field: {}. Please ensure you're uploading a valid image file.", e)
            )
        })?
    {
        field_count += 1;
        let field_name = field.name().unwrap_or("").to_string();
        tracing::debug!("Processing multipart field #{}: '{}'", field_count, field_name);

        if field_name == "image" {
            mime_type = field.content_type().map(|ct| ct.to_string());
            tracing::debug!("Image field found with content_type: {:?}", mime_type);

            let data = field.bytes().await.map_err(|e| {
                tracing::error!("Failed to read image bytes: {}", e);
                AppError::BadRequest(
                    format!("Failed to read image data: {}. The image may be corrupted.", e)
                )
            })?;

            tracing::debug!("Successfully read {} bytes from image field", data.len());
            image_data = Some(data.to_vec());
        }
    }

    tracing::debug!("Processed {} multipart fields total", field_count);

    let image_data = image_data.ok_or_else(|| {
        AppError::BadRequest("No image provided. Please upload an image file.".to_string())
    })?;

    if image_data.len() > 20 * 1024 * 1024 {
        return Err(AppError::BadRequest("Image too large. Maximum size is 20MB.".to_string()));
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
            let error_msg = e.to_string();
            if
                error_msg.contains("SAFETY") ||
                error_msg.contains("blocked") ||
                error_msg.contains("filter")
            {
                return AppError::BadRequest(
                    "This image could not be processed. Please upload a clear photo of food.".to_string()
                );
            }
            AppError::InternalError(e)
        })?;

    tracing::info!("Successfully analyzed food image");

    let (is_valid_food, error_type, message) = parse_validation_response(&analysis);

    let response = NutritionAnalysisResponse {
        success: true,
        analysis,
        is_valid_food: Some(is_valid_food),
        error_type,
        message,
        timestamp: chrono::Utc::now(),
    };

    Ok((StatusCode::OK, Json(response)))
}

fn parse_validation_response(analysis: &str) -> (bool, Option<String>, Option<String>) {
    if let Some(start) = analysis.find('{') {
        if let Some(end) = analysis.rfind('}') {
            let json_str = &analysis[start..=end];
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_str) {
                let is_valid = parsed
                    .get("is_valid_food")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);

                if !is_valid {
                    let error_type = parsed
                        .get("error_type")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let message = parsed
                        .get("message")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    return (false, error_type, message);
                }
            }
        }
    }
    (true, None, None)
}

pub async fn quick_food_check(
    State(state): State<AppState>,
    mut multipart: Multipart
) -> Result<impl IntoResponse, AppError> {
    tracing::info!("Received request for quick food check");

    let mut image_data: Option<Vec<u8>> = None;
    let mut mime_type: Option<String> = None;
    let mut field_count = 0;

    while
        let Some(field) = multipart.next_field().await.map_err(|e| {
            tracing::error!("Multipart parsing error: {}", e);
            AppError::BadRequest(
                format!("Failed to read multipart field: {}. Please ensure you're uploading a valid image file.", e)
            )
        })?
    {
        field_count += 1;
        let field_name = field.name().unwrap_or("").to_string();
        tracing::debug!("Processing multipart field #{}: '{}'", field_count, field_name);

        if field_name == "image" {
            mime_type = field.content_type().map(|ct| ct.to_string());
            tracing::debug!("Image field found with content_type: {:?}", mime_type);

            let data = field.bytes().await.map_err(|e| {
                tracing::error!("Failed to read image bytes: {}", e);
                AppError::BadRequest(
                    format!("Failed to read image data: {}. The image may be corrupted.", e)
                )
            })?;

            tracing::debug!("Successfully read {} bytes from image field", data.len());
            image_data = Some(data.to_vec());
        }
    }

    let image_data = image_data.ok_or_else(|| {
        AppError::BadRequest("No image provided. Please upload an image file.".to_string())
    })?;

    if image_data.len() > 20 * 1024 * 1024 {
        return Err(AppError::BadRequest("Image too large. Maximum size is 20MB.".to_string()));
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

pub async fn analyze_food_text(
    State(state): State<AppState>,
    Json(payload): Json<FoodTextRequest>
) -> Result<impl IntoResponse, AppError> {
    tracing::info!("Received request for text-based food analysis: {}", payload.food_description);

    if payload.food_description.trim().is_empty() {
        return Err(AppError::BadRequest("Food description cannot be empty".to_string()));
    }

    let nutrition_data = state.gemini_service
        .analyze_food_from_text(&payload.food_description).await
        .map_err(|e| {
            tracing::error!("Gemini API error: {}", e);
            AppError::InternalError(e)
        })?;

    tracing::info!("Successfully analyzed food from text");

    Ok((StatusCode::OK, Json(nutrition_data)))
}
