use axum::{
    extract::{ Path, Query, State },
    http::StatusCode,
    response::IntoResponse,
    Extension,
    Json,
};
use chrono::{ DateTime, NaiveDate, Utc, TimeZone };
use mongodb::bson::{ doc, oid::ObjectId };
use serde::{ Deserialize, Serialize };
use futures::TryStreamExt;

use crate::{ db::AppState, error::AppError, models::* };

#[derive(Debug, Deserialize)]
pub struct LogMealRequest {
    pub meal_type: MealType,
    pub food_name: String,
    pub calories: f64,
    pub protein_g: f64,
    pub carbs_g: f64,
    pub fat_g: f64,
    pub serving_size: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MealLogResponse {
    pub success: bool,
    pub meal: MealLog,
    pub daily_totals: DailyTotals,
}

#[derive(Debug, Serialize)]
pub struct DailyTotals {
    pub total_calories: f64,
    pub total_protein_g: f64,
    pub total_carbs_g: f64,
    pub total_fat_g: f64,
    pub target_calories: f64,
    pub target_protein_g: f64,
    pub target_carbs_g: f64,
    pub target_fat_g: f64,
    pub calories_remaining: f64,
    pub protein_remaining: f64,
    pub carbs_remaining: f64,
    pub fat_remaining: f64,
}

#[derive(Debug, Deserialize)]
pub struct DateQuery {
    pub date: Option<String>, }


pub async fn log_meal(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<LogMealRequest>
) -> Result<impl IntoResponse, AppError> {
    let user_id = ObjectId::parse_str(&claims.sub).map_err(|_|
        AppError::BadRequest("Invalid user ID".to_string())
    )?;

    tracing::info!("Logging meal for user: {}", claims.sub);

    let now = Utc::now();
    tracing::info!("Current UTC time: {}", now);

    let meal_log = MealLog {
        id: None,
        user_id,
        date: now,
        meal_type: payload.meal_type,
        food_name: payload.food_name.clone(),
        calories: payload.calories,
        protein_g: payload.protein_g,
        carbs_g: payload.carbs_g,
        fat_g: payload.fat_g,
        serving_size: payload.serving_size.clone(),
        notes: payload.notes.clone(),
        created_at: now,
    };

    tracing::info!(
        "Meal log before insert - date: {:?}, food: {}",
        meal_log.date,
        meal_log.food_name
    );

    let result = state.db
        .collection::<MealLog>("meal_logs")
        .insert_one(&meal_log, None).await
        .map_err(|e| {
            tracing::error!("Failed to insert meal log: {}", e);
            AppError::InternalError(e.into())
        })?;

    let mut saved_meal = meal_log;
    saved_meal.id = Some(result.inserted_id.as_object_id().unwrap());

    tracing::info!("Meal inserted with ID: {:?}, date: {:?}", saved_meal.id, saved_meal.date);

    let daily_totals = calculate_daily_totals(&state, user_id, Utc::now()).await?;

    tracing::info!("Meal logged successfully for user: {}", claims.sub);

    Ok((
        StatusCode::CREATED,
        Json(MealLogResponse {
            success: true,
            meal: saved_meal,
            daily_totals,
        }),
    ))
}


pub async fn get_daily_meals(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(query): Query<DateQuery>
) -> Result<impl IntoResponse, AppError> {
    let user_id = ObjectId::parse_str(&claims.sub).map_err(|_|
        AppError::BadRequest("Invalid user ID".to_string())
    )?;

    let naive_date = if let Some(date_str) = query.date {
        NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
            .map_err(|_| AppError::BadRequest("Invalid date format. Use YYYY-MM-DD".to_string()))?
    } else {
        Utc::now().date_naive()
    };

    let start_of_day = naive_date
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| AppError::BadRequest("Invalid date".to_string()))?;
    let start_of_day = Utc.from_utc_datetime(&start_of_day);
    
    let end_of_day = start_of_day + chrono::Duration::days(1);

    tracing::info!(
        "Fetching meals for user {} on date {} (start: {}, end: {})",
        claims.sub,
        naive_date.format("%Y-%m-%d"),
        start_of_day,
        end_of_day
    );

    use futures::TryStreamExt;
    let all_meals_cursor = state.db
        .collection::<MealLog>("meal_logs")
        .find(doc! { "user_id": user_id }, None).await
        .map_err(|e| AppError::InternalError(e.into()))?;

    let all_meals: Vec<MealLog> = all_meals_cursor
        .try_collect().await
        .map_err(|e| AppError::InternalError(e.into()))?;

    tracing::info!("Total meals in DB for user: {}", all_meals.len());
    for meal in &all_meals {
        tracing::info!("  Meal: id={:?}, date={:?}, food={}", meal.id, meal.date, meal.food_name);
    }

    let start_bson = mongodb::bson::DateTime::from_chrono(start_of_day);
    let end_bson = mongodb::bson::DateTime::from_chrono(end_of_day);

    let filter =
        doc! {
        "user_id": user_id,
        "date": {
            "$gte": start_bson,
            "$lt": end_bson
        }
    };

    tracing::info!("Query filter: {:?}", filter);
    tracing::info!("Looking for meals between {} and {}", start_bson, end_bson);

    let mut cursor = state.db
        .collection::<MealLog>("meal_logs")
        .find(filter, None).await
        .map_err(|e| AppError::InternalError(e.into()))?;

    let mut meals = Vec::new();
    while cursor.advance().await.map_err(|e| AppError::InternalError(e.into()))? {
        let meal = cursor.deserialize_current().map_err(|e| {
            tracing::error!("Failed to deserialize meal: {}", e);
            AppError::InternalError(e.into())
        })?;
        tracing::info!("Found meal: {:?}", meal);
        meals.push(meal);
    }

    tracing::info!("Total meals found with date query: {}", meals.len());

    if meals.is_empty() && !all_meals.is_empty() {
        tracing::warn!("No meals found with date query, filtering manually from all meals");
        meals = all_meals.into_iter()
            .filter(|meal| {
                let meal_date = meal.date;
                let in_range = meal_date >= start_of_day && meal_date < end_of_day;
                if in_range {
                    tracing::info!("Meal {} is in range: {}", meal.food_name, meal_date);
                }
                in_range
            })
            .collect();
        tracing::info!("Manually filtered meals: {}", meals.len());
    }

    let daily_totals = calculate_daily_totals(&state, user_id, start_of_day).await?;

    Ok(
        Json(
            serde_json::json!({
        "meals": meals,
        "daily_totals": daily_totals,
        "date": naive_date.format("%Y-%m-%d").to_string(),
    })
        )
    )
}


pub async fn update_meal(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(meal_id): Path<String>,
    Json(payload): Json<LogMealRequest>
) -> Result<impl IntoResponse, AppError> {
    let user_id = ObjectId::parse_str(&claims.sub).map_err(|_|
        AppError::BadRequest("Invalid user ID".to_string())
    )?;

    let meal_oid = ObjectId::parse_str(&meal_id).map_err(|_|
        AppError::BadRequest("Invalid meal ID".to_string())
    )?;

    tracing::info!("Updating meal {} for user {}", meal_id, claims.sub);

    let update_doc =
        doc! {
        "$set": {
            "meal_type": mongodb::bson::to_bson(&payload.meal_type).unwrap(),
            "food_name": &payload.food_name,
            "calories": payload.calories,
            "protein_g": payload.protein_g,
            "carbs_g": payload.carbs_g,
            "fat_g": payload.fat_g,
            "serving_size": &payload.serving_size,
            "notes": &payload.notes,
        }
    };

    let result = state.db
        .collection::<MealLog>("meal_logs")
        .update_one(
            doc! {
                "_id": meal_oid,
                "user_id": user_id
            },
            update_doc,
            None
        ).await
        .map_err(|e| AppError::InternalError(e.into()))?;

    if result.matched_count == 0 {
        return Err(AppError::NotFound("Meal not found".to_string()));
    }

    let updated_meal = state.db
        .collection::<MealLog>("meal_logs")
        .find_one(doc! { "_id": meal_oid }, None).await
        .map_err(|e| AppError::InternalError(e.into()))?
        .ok_or_else(|| AppError::NotFound("Meal not found".to_string()))?;

    let daily_totals = calculate_daily_totals(&state, user_id, updated_meal.date).await?;

    Ok(
        Json(MealLogResponse {
            success: true,
            meal: updated_meal,
            daily_totals,
        })
    )
}


pub async fn delete_meal(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(meal_id): Path<String>
) -> Result<impl IntoResponse, AppError> {
    let user_id = ObjectId::parse_str(&claims.sub).map_err(|_|
        AppError::BadRequest("Invalid user ID".to_string())
    )?;

    let meal_oid = ObjectId::parse_str(&meal_id).map_err(|_|
        AppError::BadRequest("Invalid meal ID".to_string())
    )?;

    tracing::info!("Deleting meal {} for user {}", meal_id, claims.sub);

    let meal = state.db
        .collection::<MealLog>("meal_logs")
        .find_one(
            doc! {
                "_id": meal_oid,
                "user_id": user_id
            },
            None
        ).await
        .map_err(|e| AppError::InternalError(e.into()))?
        .ok_or_else(|| AppError::NotFound("Meal not found".to_string()))?;

    let meal_date = meal.date;

    let result = state.db
        .collection::<MealLog>("meal_logs")
        .delete_one(
            doc! {
                "_id": meal_oid,
                "user_id": user_id
            },
            None
        ).await
        .map_err(|e| AppError::InternalError(e.into()))?;

    if result.deleted_count == 0 {
        return Err(AppError::NotFound("Meal not found".to_string()));
    }

    let daily_totals = calculate_daily_totals(&state, user_id, meal_date).await?;

    Ok(
        Json(
            serde_json::json!({
        "success": true,
        "message": "Meal deleted successfully",
        "daily_totals": daily_totals,
    })
        )
    )
}

async fn calculate_daily_totals(
    state: &AppState,
    user_id: ObjectId,
    date: DateTime<Utc>
) -> Result<DailyTotals, AppError> {
    let start_of_day = date
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| AppError::InternalError(anyhow::anyhow!("Invalid date")))?;
    let start_of_day = Utc.from_utc_datetime(&start_of_day);
    let end_of_day = start_of_day + chrono::Duration::days(1);

    let start_bson = mongodb::bson::DateTime::from_chrono(start_of_day);
    let end_bson = mongodb::bson::DateTime::from_chrono(end_of_day);

    use futures::TryStreamExt;
    let all_meals_cursor = state.db
        .collection::<MealLog>("meal_logs")
        .find(doc! { "user_id": user_id }, None).await
        .map_err(|e| AppError::InternalError(e.into()))?;

    let all_meals: Vec<MealLog> = all_meals_cursor
        .try_collect().await
        .map_err(|e| AppError::InternalError(e.into()))?;

    tracing::info!("calculate_daily_totals: Total meals in DB for user: {}", all_meals.len());

    let mut cursor = state.db
        .collection::<MealLog>("meal_logs")
        .find(
            doc! {
                "user_id": user_id,
                "date": {
                    "$gte": start_bson,
                    "$lt": end_bson
                }
            },
            None
        ).await
        .map_err(|e| AppError::InternalError(e.into()))?;

    let mut meals_in_range = Vec::new();
    while cursor.advance().await.map_err(|e| AppError::InternalError(e.into()))? {
        let meal: MealLog = cursor
            .deserialize_current()
            .map_err(|e| AppError::InternalError(e.into()))?;
        meals_in_range.push(meal);
    }

    tracing::info!("calculate_daily_totals: Found {} meals with date query", meals_in_range.len());

    if meals_in_range.is_empty() && !all_meals.is_empty() {
        tracing::warn!("calculate_daily_totals: No meals found with date query, filtering manually");
        meals_in_range = all_meals.into_iter()
            .filter(|meal| {
                let meal_date = meal.date;
                meal_date >= start_of_day && meal_date < end_of_day
            })
            .collect();
        tracing::info!("calculate_daily_totals: Manually filtered {} meals", meals_in_range.len());
    }

    let mut total_calories = 0.0;
    let mut total_protein = 0.0;
    let mut total_carbs = 0.0;
    let mut total_fat = 0.0;

    for meal in meals_in_range {
        tracing::info!("Including meal in totals: {} - {}cal", meal.food_name, meal.calories);
        total_calories += meal.calories;
        total_protein += meal.protein_g;
        total_carbs += meal.carbs_g;
        total_fat += meal.fat_g;
    }

    tracing::info!("calculate_daily_totals: Totals - calories: {}, protein: {}, carbs: {}, fat: {}", 
        total_calories, total_protein, total_carbs, total_fat);

    let user = state.db
        .collection::<User>("users")
        .find_one(doc! { "_id": user_id }, None).await
        .map_err(|e| AppError::InternalError(e.into()))?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    let (target_calories, target_protein, target_carbs, target_fat) = if
        let Some(profile) = user.health_profile
    {
        (
            profile.daily_calories,
            profile.daily_protein_g,
            profile.daily_carbs_g,
            profile.daily_fat_g,
        )
    } else {

        (2000.0, 150.0, 250.0, 67.0)
    };

    Ok(DailyTotals {
        total_calories,
        total_protein_g: total_protein,
        total_carbs_g: total_carbs,
        total_fat_g: total_fat,
        target_calories,
        target_protein_g: target_protein,
        target_carbs_g: target_carbs,
        target_fat_g: target_fat,
        calories_remaining: target_calories - total_calories,
        protein_remaining: target_protein - total_protein,
        carbs_remaining: target_carbs - total_carbs,
        fat_remaining: target_fat - total_fat,
    })
}

#[derive(Debug, Deserialize)]
pub struct PeriodQuery {
    pub start_date: String,
    pub end_date: String,
}

#[derive(Debug, Serialize)]
pub struct PeriodStatsResponse {
    pub success: bool,
    pub period_type: String,
    pub start_date: String,
    pub end_date: String,
    pub daily_data: Vec<DailyDataPoint>,
    pub averages: PeriodAverages,
    pub totals: PeriodTotals,
    pub goal_progress: GoalProgress,
}

#[derive(Debug, Serialize)]
pub struct DailyDataPoint {
    pub date: String,
    pub calories: f64,
    pub protein_g: f64,
    pub carbs_g: f64,
    pub fat_g: f64,
    pub meal_count: usize,
}

#[derive(Debug, Serialize)]
pub struct PeriodAverages {
    pub avg_calories: f64,
    pub avg_protein_g: f64,
    pub avg_carbs_g: f64,
    pub avg_fat_g: f64,
    pub avg_meals_per_day: f64,
}

#[derive(Debug, Serialize)]
pub struct PeriodTotals {
    pub total_calories: f64,
    pub total_protein_g: f64,
    pub total_carbs_g: f64,
    pub total_fat_g: f64,
    pub total_meals: usize,
    pub days_logged: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeightGoalInfo {
    pub starting_weight: f64,
    pub goal_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GoalProgress {
    pub target_calories: f64,
    pub target_protein_g: f64,
    pub target_carbs_g: f64,
    pub target_fat_g: f64,
    pub calories_compliance_percent: f64,
    pub protein_compliance_percent: f64,
    pub carbs_compliance_percent: f64,
    pub fat_compliance_percent: f64,
    pub days_on_target: usize,
    pub total_days: usize,
    pub goal_type: String,
    pub estimated_progress: Option<f64>,
    pub weight_goal: Option<WeightGoalInfo>,
    pub current_weight: Option<f64>,
    pub target_weight: Option<f64>,
}

pub async fn get_period_stats(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(query): Query<PeriodQuery>
) -> Result<impl IntoResponse, AppError> {
    let user_id = ObjectId::parse_str(&claims.sub).map_err(|_|
        AppError::BadRequest("Invalid user ID".to_string())
    )?;

    let start_date = NaiveDate
        ::parse_from_str(&query.start_date, "%Y-%m-%d")
        .map_err(|_| AppError::BadRequest("Invalid start_date format".to_string()))?;

    let end_date = NaiveDate
        ::parse_from_str(&query.end_date, "%Y-%m-%d")
        .map_err(|_| AppError::BadRequest("Invalid end_date format".to_string()))?;

    tracing::info!("Fetching period stats for user {} from {} to {}", claims.sub, start_date, end_date);

    let start_datetime = Utc.from_utc_datetime(&start_date.and_hms_opt(0, 0, 0).unwrap());
    let end_datetime = Utc.from_utc_datetime(&end_date.and_hms_opt(23, 59, 59).unwrap());

    let start_bson = mongodb::bson::DateTime::from_chrono(start_datetime);
    let end_bson = mongodb::bson::DateTime::from_chrono(end_datetime);

    let filter = doc! {
        "user_id": user_id,
        "date": {
            "$gte": start_bson,
            "$lte": end_bson
        }
    };

    let mut cursor = state.db
        .collection::<MealLog>("meal_logs")
        .find(filter, None).await
        .map_err(|e| {
            tracing::error!("Failed to query meals for period: {}", e);
            AppError::InternalError(e.into())
        })?;

    let mut all_meals: Vec<MealLog> = Vec::new();
    while
        let Some(result) = cursor.try_next().await.map_err(|e| {
            tracing::error!("Error iterating cursor: {}", e);
            AppError::InternalError(e.into())
        })?
    {
        all_meals.push(result);
    }

    if all_meals.is_empty() {
        tracing::warn!("No meals found in period, trying manual filtering");
        let all_meals_filter = doc! { "user_id": user_id };
        let mut all_cursor = state.db
            .collection::<MealLog>("meal_logs")
            .find(all_meals_filter, None).await
            .map_err(|e| AppError::InternalError(e.into()))?;

        while
            let Some(result) = all_cursor.try_next().await.map_err(|e| {
                AppError::InternalError(e.into())
            })?
        {
            let meal_date = result.date;
            if meal_date >= start_datetime && meal_date <= end_datetime {
                all_meals.push(result);
            }
        }
    }

    tracing::info!("Found {} meals in period", all_meals.len());

    use std::collections::HashMap;
    let mut daily_map: HashMap<String, Vec<&MealLog>> = HashMap::new();

    for meal in &all_meals {
        let date_str = meal.date.format("%Y-%m-%d").to_string();
        daily_map.entry(date_str).or_insert_with(Vec::new).push(meal);
    }

    let mut daily_data: Vec<DailyDataPoint> = Vec::new();
    let mut current_date = start_date;

    while current_date <= end_date {
        let date_str = current_date.format("%Y-%m-%d").to_string();
        let meals_for_day = daily_map.get(&date_str).cloned().unwrap_or_default();

        let (calories, protein, carbs, fat) = meals_for_day
            .iter()
            .fold((0.0, 0.0, 0.0, 0.0), |(c, p, cr, f), meal| {
                (c + meal.calories, p + meal.protein_g, cr + meal.carbs_g, f + meal.fat_g)
            });

        daily_data.push(DailyDataPoint {
            date: date_str,
            calories,
            protein_g: protein,
            carbs_g: carbs,
            fat_g: fat,
            meal_count: meals_for_day.len(),
        });

        current_date = current_date.succ_opt().unwrap();
    }

    let days_with_meals = daily_data.iter().filter(|d| d.meal_count > 0).count();
    let total_days = daily_data.len();

    let totals = PeriodTotals {
        total_calories: daily_data.iter().map(|d| d.calories).sum(),
        total_protein_g: daily_data.iter().map(|d| d.protein_g).sum(),
        total_carbs_g: daily_data.iter().map(|d| d.carbs_g).sum(),
        total_fat_g: daily_data.iter().map(|d| d.fat_g).sum(),
        total_meals: all_meals.len(),
        days_logged: days_with_meals,
    };

    let averages = if days_with_meals > 0 {
        PeriodAverages {
            avg_calories: totals.total_calories / (days_with_meals as f64),
            avg_protein_g: totals.total_protein_g / (days_with_meals as f64),
            avg_carbs_g: totals.total_carbs_g / (days_with_meals as f64),
            avg_fat_g: totals.total_fat_g / (days_with_meals as f64),
            avg_meals_per_day: (totals.total_meals as f64) / (days_with_meals as f64),
        }
    } else {
        PeriodAverages {
            avg_calories: 0.0,
            avg_protein_g: 0.0,
            avg_carbs_g: 0.0,
            avg_fat_g: 0.0,
            avg_meals_per_day: 0.0,
        }
    };

    let user = state.db
        .collection::<User>("users")
        .find_one(doc! { "_id": user_id }, None).await
        .map_err(|e| AppError::InternalError(e.into()))?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    let (target_calories, target_protein, target_carbs, target_fat, goal_type, estimated_progress, weight_goal, current_weight, target_weight) = if
        let Some(profile) = user.health_profile
    {
        let goal = match profile.goal {
            crate::models::HealthGoal::LoseWeight => "lose_weight".to_string(),
            crate::models::HealthGoal::MaintainWeight => "maintain_weight".to_string(),
            crate::models::HealthGoal::GainWeight => "gain_weight".to_string(),
            crate::models::HealthGoal::BuildMuscle => "build_muscle".to_string(),
        };
        
        let estimated = if days_with_meals > 7 {
            let avg_cal_diff = averages.avg_calories - profile.daily_calories;
            let days_elapsed = days_with_meals as f64;
            let calories_per_kg = 7700.0;
            let estimated_weight_change = (avg_cal_diff * days_elapsed) / calories_per_kg;
            Some(estimated_weight_change)
        } else {
            None
        };

        let weight_goal_data = Some(WeightGoalInfo {
            starting_weight: profile.weight_kg,
            goal_type: goal.clone(),
        });

        let current_wt = Some(profile.weight_kg);
        
        let target_wt = match profile.goal {
            crate::models::HealthGoal::LoseWeight => Some(profile.weight_kg * 0.9), 
            crate::models::HealthGoal::GainWeight => Some(profile.weight_kg * 1.1), 
            crate::models::HealthGoal::BuildMuscle => Some(profile.weight_kg * 1.05), 
            crate::models::HealthGoal::MaintainWeight => Some(profile.weight_kg),
        };

        (
            profile.daily_calories,
            profile.daily_protein_g,
            profile.daily_carbs_g,
            profile.daily_fat_g,
            goal,
            estimated,
            weight_goal_data,
            current_wt,
            target_wt,
        )
    } else {
        (2000.0, 150.0, 250.0, 67.0, "maintain_weight".to_string(), None, None, None, None)
    };

    let days_on_target = daily_data
        .iter()
        .filter(|d| {
            let cal_diff = (d.calories - target_calories).abs();
            cal_diff / target_calories <= 0.1 && d.meal_count > 0
        })
        .count();

    let goal_progress = GoalProgress {
        target_calories,
        target_protein_g: target_protein,
        target_carbs_g: target_carbs,
        target_fat_g: target_fat,
        calories_compliance_percent: if days_with_meals > 0 {
            ((days_on_target as f64) / (days_with_meals as f64)) * 100.0
        } else {
            0.0
        },
        protein_compliance_percent: if days_with_meals > 0 && target_protein > 0.0 {
            (averages.avg_protein_g / target_protein) * 100.0
        } else {
            0.0
        },
        carbs_compliance_percent: if days_with_meals > 0 && target_carbs > 0.0 {
            (averages.avg_carbs_g / target_carbs) * 100.0
        } else {
            0.0
        },
        fat_compliance_percent: if days_with_meals > 0 && target_fat > 0.0 {
            (averages.avg_fat_g / target_fat) * 100.0
        } else {
            0.0
        },
        days_on_target,
        total_days: days_with_meals,
        goal_type,
        estimated_progress,
        weight_goal,
        current_weight,
        target_weight,
    };

    let period_type = if total_days <= 7 {
        "week".to_string()
    } else if total_days <= 31 {
        "month".to_string()
    } else {
        "year".to_string()
    };

    Ok(
        Json(PeriodStatsResponse {
            success: true,
            period_type,
            start_date: query.start_date,
            end_date: query.end_date,
            daily_data,
            averages,
            totals,
            goal_progress,
        })
    )
}
