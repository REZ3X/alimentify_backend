use axum::{
    extract::{Path, Query, State},
    response::{IntoResponse, Json},
    Extension,
};
use mongodb::bson::{doc, oid::ObjectId};
use serde::{Deserialize, Serialize};
use chrono::Utc;
use futures::stream::TryStreamExt;

use crate::{
    db::AppState,
    error::AppError,
    models::{Claims, MealReport, ReportPeriod, ReportStatus, User, MealLog},
    services::email_service::EmailService,
};

#[derive(Debug, Deserialize)]
pub struct GenerateReportQuery {
    pub report_type: String, 
    pub start_date: String,
    pub end_date: String,
    #[serde(default)]
    pub send_email: bool,
}

#[derive(Debug, Serialize)]
pub struct ReportResponse {
    pub success: bool,
    pub report: MealReport,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ReportsListResponse {
    pub success: bool,
    pub reports: Vec<MealReport>,
    pub total: usize,
}

pub async fn generate_report(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(query): Query<GenerateReportQuery>,
) -> Result<impl IntoResponse, AppError> {
    let user_id = ObjectId::parse_str(&claims.sub)
        .map_err(|_| AppError::BadRequest("Invalid user ID".to_string()))?;

    let user = state.db
        .collection::<User>("users")
        .find_one(doc! { "_id": user_id }, None)
        .await
        .map_err(|e| AppError::InternalError(e.into()))?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    let report_type = match query.report_type.to_lowercase().as_str() {
        "daily" => ReportPeriod::Daily,
        "weekly" => ReportPeriod::Weekly,
        "monthly" => ReportPeriod::Monthly,
        "yearly" => ReportPeriod::Yearly,
        _ => return Err(AppError::BadRequest("Invalid report type".to_string())),
    };

    let start_date = chrono::NaiveDate::parse_from_str(&query.start_date, "%Y-%m-%d")
        .map_err(|_| AppError::BadRequest("Invalid start date format".to_string()))?;
    let end_date = chrono::NaiveDate::parse_from_str(&query.end_date, "%Y-%m-%d")
        .map_err(|_| AppError::BadRequest("Invalid end date format".to_string()))?;

    let start_datetime = chrono::TimeZone::from_utc_datetime(&chrono::Utc, &start_date.and_hms_opt(0, 0, 0).unwrap());
    let end_datetime = chrono::TimeZone::from_utc_datetime(&chrono::Utc, &end_date.and_hms_opt(23, 59, 59).unwrap());

    let start_bson = mongodb::bson::DateTime::from_chrono(start_datetime);
    let end_bson = mongodb::bson::DateTime::from_chrono(end_datetime);

    tracing::info!("Querying meals for user {} from {} to {}", user_id, start_datetime, end_datetime);

    let mut cursor = state.db
        .collection::<MealLog>("meal_logs")
        .find(
            doc! {
                "user_id": user_id,
                "date": {
                    "$gte": start_bson,
                    "$lte": end_bson,
                }
            },
            None,
        )
        .await
        .map_err(|e| AppError::InternalError(e.into()))?;

    let mut meals: Vec<MealLog> = Vec::new();
    while let Some(meal) = cursor.try_next().await.map_err(|e| AppError::InternalError(e.into()))? {
        tracing::debug!("Found meal: {} on {}", meal.food_name, meal.date);
        meals.push(meal);
    }
    
    tracing::info!("Found {} meals with BSON date query for report", meals.len());

    if meals.is_empty() {
        tracing::warn!("No meals found with BSON query, trying manual filtering");
        let all_meals_cursor = state.db
            .collection::<MealLog>("meal_logs")
            .find(doc! { "user_id": user_id }, None)
            .await
            .map_err(|e| AppError::InternalError(e.into()))?;

        let all_meals: Vec<MealLog> = all_meals_cursor
            .try_collect()
            .await
            .map_err(|e| AppError::InternalError(e.into()))?;

        tracing::info!("Total meals in DB for user: {}", all_meals.len());
        
        meals = all_meals.into_iter()
            .filter(|meal| {
                let meal_date = meal.date;
                let in_range = meal_date >= start_datetime && meal_date <= end_datetime;
                if in_range {
                    tracing::debug!("Meal {} is in range: {}", meal.food_name, meal_date);
                }
                in_range
            })
            .collect();
        
        tracing::info!("Manually filtered {} meals for report", meals.len());
    }

    let total_days = (end_date - start_date).num_days() as usize + 1;
    let mut days_with_meals = std::collections::HashSet::new();
    let mut total_calories = 0.0;
    let mut total_protein = 0.0;
    let mut total_carbs = 0.0;
    let mut total_fat = 0.0;

    for meal in &meals {
        days_with_meals.insert(meal.date.date_naive());
        total_calories += meal.calories;
        total_protein += meal.protein_g;
        total_carbs += meal.carbs_g;
        total_fat += meal.fat_g;
    }

    let days_logged = days_with_meals.len();
    let avg_calories = if days_logged > 0 { total_calories / days_logged as f64 } else { 0.0 };
    let avg_protein = if days_logged > 0 { total_protein / days_logged as f64 } else { 0.0 };
    let avg_carbs = if days_logged > 0 { total_carbs / days_logged as f64 } else { 0.0 };
    let avg_fat = if days_logged > 0 { total_fat / days_logged as f64 } else { 0.0 };

    let (target_calories, target_protein, target_carbs, target_fat, goal_type) = if let Some(profile) = &user.health_profile {
        let goal = match profile.goal {
            crate::models::HealthGoal::LoseWeight => "lose_weight".to_string(),
            crate::models::HealthGoal::MaintainWeight => "maintain_weight".to_string(),
            crate::models::HealthGoal::GainWeight => "gain_weight".to_string(),
            crate::models::HealthGoal::BuildMuscle => "build_muscle".to_string(),
        };
        (
            profile.daily_calories,
            profile.daily_protein_g,
            profile.daily_carbs_g,
            profile.daily_fat_g,
            goal,
        )
    } else {
        (2000.0, 150.0, 250.0, 67.0, "maintain_weight".to_string())
    };

    let calories_compliance = if target_calories > 0.0 {
        (avg_calories / target_calories * 100.0).min(100.0)
    } else {
        0.0
    };
    let protein_compliance = if target_protein > 0.0 {
        (avg_protein / target_protein * 100.0).min(100.0)
    } else {
        0.0
    };
    let carbs_compliance = if target_carbs > 0.0 {
        (avg_carbs / target_carbs * 100.0).min(100.0)
    } else {
        0.0
    };
    let fat_compliance = if target_fat > 0.0 {
        (avg_fat / target_fat * 100.0).min(100.0)
    } else {
        0.0
    };

    let days_on_target = days_with_meals.iter().filter(|date| {
        let day_meals: Vec<&MealLog> = meals.iter()
            .filter(|m| m.date.date_naive() == **date)
            .collect();
        let day_calories: f64 = day_meals.iter().map(|m| m.calories).sum();
        let diff = (day_calories - target_calories).abs();
        diff / target_calories <= 0.1
    }).count();

    let avg_compliance = (calories_compliance + protein_compliance + carbs_compliance + fat_compliance) / 4.0;
    let goal_achieved = avg_compliance >= 80.0 && days_logged as f64 / total_days as f64 >= 0.7;

    let mut best_day_date = None;
    let mut best_day_compliance = 0.0;
    for date in &days_with_meals {
        let day_meals: Vec<&MealLog> = meals.iter()
            .filter(|m| m.date.date_naive() == *date)
            .collect();
        let day_calories: f64 = day_meals.iter().map(|m| m.calories).sum();
        let day_protein: f64 = day_meals.iter().map(|m| m.protein_g).sum();
        let day_carbs: f64 = day_meals.iter().map(|m| m.carbs_g).sum();
        let day_fat: f64 = day_meals.iter().map(|m| m.fat_g).sum();

        let day_cal_comp = (day_calories / target_calories * 100.0).min(100.0);
        let day_prot_comp = (day_protein / target_protein * 100.0).min(100.0);
        let day_carb_comp = (day_carbs / target_carbs * 100.0).min(100.0);
        let day_fat_comp = (day_fat / target_fat * 100.0).min(100.0);
        let day_avg_comp = (day_cal_comp + day_prot_comp + day_carb_comp + day_fat_comp) / 4.0;

        if day_avg_comp > best_day_compliance {
            best_day_compliance = day_avg_comp;
            best_day_date = Some(date.format("%Y-%m-%d").to_string());
        }
    }

    let mut streak = 0;
    let mut current_streak = 0;
    let mut last_date: Option<chrono::NaiveDate> = None;
    let mut sorted_dates: Vec<_> = days_with_meals.iter().collect();
    sorted_dates.sort();
    
    for date in sorted_dates {
        if let Some(last) = last_date {
            if (*date - last).num_days() == 1 {
                current_streak += 1;
            } else {
                streak = streak.max(current_streak);
                current_streak = 1;
            }
        } else {
            current_streak = 1;
        }
        last_date = Some(*date);
    }
    streak = streak.max(current_streak);

    let (starting_weight, ending_weight, weight_change, target_weight, weight_goal_achieved) = 
        if let Some(profile) = &user.health_profile {
            let starting = Some(profile.weight_kg);
            let target = match profile.goal {
                crate::models::HealthGoal::LoseWeight => Some(profile.weight_kg * 0.9),
                crate::models::HealthGoal::GainWeight => Some(profile.weight_kg * 1.1),
                crate::models::HealthGoal::BuildMuscle => Some(profile.weight_kg * 1.05),
                crate::models::HealthGoal::MaintainWeight => Some(profile.weight_kg),
            };
            (starting, starting, Some(0.0), target, Some(false))
        } else {
            (None, None, None, None, None)
        };

    let report = MealReport {
        id: None,
        user_id,
        report_type: report_type.clone(),
        start_date: query.start_date.clone(),
        end_date: query.end_date.clone(),
        generated_at: Utc::now(),
        status: if query.send_email { ReportStatus::Sent } else { ReportStatus::Generated },
        total_days,
        days_logged,
        total_meals: meals.len(),
        avg_calories,
        avg_protein_g: avg_protein,
        avg_carbs_g: avg_carbs,
        avg_fat_g: avg_fat,
        goal_type: goal_type.clone(),
        goal_achieved,
        calories_compliance_percent: calories_compliance,
        protein_compliance_percent: protein_compliance,
        carbs_compliance_percent: carbs_compliance,
        fat_compliance_percent: fat_compliance,
        days_on_target,
        starting_weight,
        ending_weight,
        weight_change,
        target_weight,
        weight_goal_achieved,
        best_day_date,
        best_day_compliance: if best_day_compliance > 0.0 { Some(best_day_compliance) } else { None },
        streak_days: streak,
        notes: None,
    };

    let result = state.db
        .collection::<MealReport>("meal_reports")
        .insert_one(&report, None)
        .await
        .map_err(|e| AppError::InternalError(e.into()))?;

    let mut saved_report = report.clone();
    saved_report.id = Some(result.inserted_id.as_object_id().unwrap());

    if query.send_email {
        let email_service = EmailService::new(
            state.config.brevo.smtp_host.clone(),
            state.config.brevo.smtp_port,
            state.config.brevo.smtp_user.clone(),
            state.config.brevo.smtp_pass.clone(),
            state.config.brevo.from_email.clone(),
            state.config.brevo.from_name.clone(),
        );

        if let Err(e) = email_service.send_report_email(&user, &saved_report).await {
            tracing::error!("Failed to send report email: {}", e);
            state.db
                .collection::<MealReport>("meal_reports")
                .update_one(
                    doc! { "_id": saved_report.id.unwrap() },
                    doc! { "$set": { "status": "Failed" } },
                    None,
                )
                .await
                .map_err(|e| AppError::InternalError(e.into()))?;
        }
    }

    Ok(Json(ReportResponse {
        success: true,
        report: saved_report,
        message: if query.send_email {
            "Report generated and sent to your email".to_string()
        } else {
            "Report generated successfully".to_string()
        },
    }))
}

pub async fn get_user_reports(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, AppError> {
    let user_id = ObjectId::parse_str(&claims.sub)
        .map_err(|_| AppError::BadRequest("Invalid user ID".to_string()))?;

    let limit = params
        .get("limit")
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(50);

    let mut cursor = state.db
        .collection::<MealReport>("meal_reports")
        .find(doc! { "user_id": user_id }, None)
        .await
        .map_err(|e| AppError::InternalError(e.into()))?;

    let mut reports: Vec<MealReport> = Vec::new();
    while let Some(report) = cursor.try_next().await.map_err(|e| AppError::InternalError(e.into()))? {
        reports.push(report);
        if reports.len() >= limit as usize {
            break;
        }
    }

    reports.sort_by(|a, b| b.generated_at.cmp(&a.generated_at));

    Ok(Json(ReportsListResponse {
        success: true,
        total: reports.len(),
        reports,
    }))
}

pub async fn get_report_by_id(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(report_id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let user_id = ObjectId::parse_str(&claims.sub)
        .map_err(|_| AppError::BadRequest("Invalid user ID".to_string()))?;
    
    let report_obj_id = ObjectId::parse_str(&report_id)
        .map_err(|_| AppError::BadRequest("Invalid report ID".to_string()))?;

    let report = state.db
        .collection::<MealReport>("meal_reports")
        .find_one(doc! { "_id": report_obj_id, "user_id": user_id }, None)
        .await
        .map_err(|e| AppError::InternalError(e.into()))?
        .ok_or_else(|| AppError::NotFound("Report not found".to_string()))?;

    Ok(Json(ReportResponse {
        success: true,
        report,
        message: "Report retrieved successfully".to_string(),
    }))
}

pub async fn delete_report(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(report_id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let user_id = ObjectId::parse_str(&claims.sub)
        .map_err(|_| AppError::BadRequest("Invalid user ID".to_string()))?;
    
    let report_obj_id = ObjectId::parse_str(&report_id)
        .map_err(|_| AppError::BadRequest("Invalid report ID".to_string()))?;

    let result = state.db
        .collection::<MealReport>("meal_reports")
        .delete_one(doc! { "_id": report_obj_id, "user_id": user_id }, None)
        .await
        .map_err(|e| AppError::InternalError(e.into()))?;

    if result.deleted_count == 0 {
        return Err(AppError::NotFound("Report not found".to_string()));
    }

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Report deleted successfully"
    })))
}
