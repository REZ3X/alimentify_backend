use axum::{ extract::{ Path, Query, State }, http::StatusCode, response::IntoResponse, Json };
use serde::{ Deserialize, Serialize };

use crate::{ db::AppState, error::AppError };

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub query: String,
    #[serde(rename = "pageNumber")]
    pub page_number: Option<i32>,
    #[serde(rename = "pageSize")]
    pub page_size: Option<i32>,
    #[serde(rename = "dataType")]
    pub data_type: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub message: Option<String>,
}

pub async fn search_foods(
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>
) -> Result<impl IntoResponse, AppError> {
    let data_types = params.data_type.map(|dt| {
        dt.split(',')
            .map(|s| s.trim().to_string())
            .collect::<Vec<String>>()
    });

    let result = state.fdc_service
        .search_foods(&params.query, params.page_number, params.page_size, data_types).await
        .map_err(|e| AppError::InternalError(e))?;

    Ok((
        StatusCode::OK,
        Json(ApiResponse {
            success: true,
            data: Some(result),
            message: None,
        }),
    ))
}

pub async fn get_food_details(
    State(state): State<AppState>,
    Path(fdc_id): Path<i32>
) -> Result<impl IntoResponse, AppError> {
    let result = state.fdc_service
        .get_food_details(fdc_id).await
        .map_err(|e| AppError::InternalError(e))?;

    Ok((
        StatusCode::OK,
        Json(ApiResponse {
            success: true,
            data: Some(result),
            message: None,
        }),
    ))
}

#[derive(Debug, Deserialize)]
pub struct GetFoodsRequest {
    #[serde(rename = "fdcIds")]
    pub fdc_ids: Vec<i32>,
}

pub async fn get_foods(
    State(state): State<AppState>,
    Json(payload): Json<GetFoodsRequest>
) -> Result<impl IntoResponse, AppError> {
    let result = state.fdc_service
        .get_foods(payload.fdc_ids).await
        .map_err(|e| AppError::InternalError(e))?;

    Ok((
        StatusCode::OK,
        Json(ApiResponse {
            success: true,
            data: Some(result),
            message: None,
        }),
    ))
}
