use axum::{ extract::{ Path, Query, State }, http::StatusCode, response::IntoResponse, Json };
use serde::{ Deserialize, Serialize };

use crate::{ db::AppState, error::AppError };

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub query: String,
}

#[derive(Debug, Deserialize)]
pub struct RandomQuery {
    #[serde(default = "default_count")]
    pub count: usize,
}

fn default_count() -> usize {
    6
}

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub message: Option<String>,
}

pub async fn search_recipes(
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>
) -> Result<impl IntoResponse, AppError> {
    let result = state.mealdb_service
        .search_meals(&params.query).await
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

pub async fn get_recipe_by_id(
    State(state): State<AppState>,
    Path(meal_id): Path<String>
) -> Result<impl IntoResponse, AppError> {
    let result = state.mealdb_service
        .get_meal_by_id(&meal_id).await
        .map_err(|e| AppError::InternalError(e))?;

    match result {
        Some(meal) =>
            Ok((
                StatusCode::OK,
                Json(ApiResponse {
                    success: true,
                    data: Some(meal),
                    message: None,
                }),
            )),
        None => Err(AppError::NotFound("Recipe not found".to_string())),
    }
}

pub async fn get_random_recipes(
    State(state): State<AppState>,
    Query(params): Query<RandomQuery>
) -> Result<impl IntoResponse, AppError> {
    let count = params.count.min(10); 

    let result = state.mealdb_service
        .get_random_meals(count).await
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

pub async fn filter_by_category(
    State(state): State<AppState>,
    Path(category): Path<String>
) -> Result<impl IntoResponse, AppError> {
    let result = state.mealdb_service
        .filter_by_category(&category).await
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

pub async fn filter_by_area(
    State(state): State<AppState>,
    Path(area): Path<String>
) -> Result<impl IntoResponse, AppError> {
    let result = state.mealdb_service
        .filter_by_area(&area).await
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
