use axum::{ extract::{ Query, State }, http::StatusCode, response::IntoResponse, Json };
use serde::{ Deserialize, Serialize };

use crate::{ db::AppState, error::AppError };

#[derive(Debug, Deserialize)]
pub struct NutritionQuery {
    pub query: String,
}

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub message: Option<String>,
}

pub async fn get_nutrition_info(
    State(state): State<AppState>,
    Query(params): Query<NutritionQuery>
) -> Result<impl IntoResponse, AppError> {
    tracing::info!("Fetching nutrition info for query: {}", params.query);

    let result = state.ninja_service.get_nutrition(&params.query).await.map_err(|e| {
        tracing::error!("Failed to get nutrition info from Ninja API: {}", e);
        AppError::ExternalApiError(
            "Nutrition data service is temporarily unavailable. Please try again later.".to_string()
        )
    })?;

    tracing::info!("Successfully retrieved {} nutrition items", result.len());

    Ok((
        StatusCode::OK,
        Json(ApiResponse {
            success: true,
            data: Some(result),
            message: None,
        }),
    ))
}
