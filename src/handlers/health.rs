use axum::{ extract::State, http::StatusCode, response::IntoResponse, Extension, Json };
use chrono::Utc;
use mongodb::bson::{ doc, oid::ObjectId };
use serde::{ Deserialize, Serialize };

use crate::{ db::AppState, error::AppError, models::* };

#[derive(Debug, Deserialize)]
pub struct CreateHealthProfileRequest {
    pub age: i32,
    pub gender: Gender,
    pub height_cm: f64,
    pub weight_kg: f64,
    pub activity_level: ActivityLevel,
    pub goal: HealthGoal,
    pub medical_conditions: Option<Vec<String>>,
    pub blood_pressure: Option<BloodPressure>,
    pub fasting_blood_sugar: Option<f64>,
    pub allergies: Option<Vec<String>>,
    pub dietary_preferences: Option<Vec<DietaryPreference>>,
}

#[derive(Debug, Serialize)]
pub struct HealthProfileResponse {
    pub success: bool,
    pub profile: HealthProfile,
    pub message: String,
}

pub async fn create_or_update_profile(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<CreateHealthProfileRequest>
) -> Result<impl IntoResponse, AppError> {
    let user_id = claims.sub;
    tracing::info!("Creating health profile for user: {}", user_id);

    let bmi = HealthProfile::calculate_bmi(payload.weight_kg, payload.height_cm);
    let bmi_category = HealthProfile::bmi_category(bmi);

    let bmr = HealthProfile::calculate_bmr(
        payload.weight_kg,
        payload.height_cm,
        payload.age,
        &payload.gender
    );

    let tdee = HealthProfile::calculate_tdee(bmr, &payload.activity_level);

    let daily_calories = HealthProfile::calculate_daily_calories(tdee, &payload.goal);

    let (protein_g, carbs_g, fat_g) = HealthProfile::calculate_macros(
        daily_calories,
        &payload.goal
    );

    let ai_prompt = format!(
        "I am a {} year old {} with the following health profile:\n\
        - Height: {:.1} cm\n\
        - Weight: {:.1} kg\n\
        - BMI: {:.1} ({})\n\
        - Activity Level: {:?}\n\
        - Goal: {:?}\n\
        - Daily Calorie Target: {:.0} kcal\n\
        - Macros: {:.0}g protein, {:.0}g carbs, {:.0}g fat\n\
        {}\n\
        {}\n\
        {}\n\n\
        Please provide:\n\
        1. Personalized nutrition recommendations\n\
        2. List of 10-15 recommended foods I should eat regularly\n\
        3. List of foods I should avoid or limit\n\
        4. General health tips\n\n\
        Format the response in clear sections.",
        payload.age,
        match payload.gender {
            Gender::Male => "male",
            Gender::Female => "female",
        },
        payload.height_cm,
        payload.weight_kg,
        bmi,
        bmi_category,
        payload.activity_level,
        payload.goal,
        daily_calories,
        protein_g,
        carbs_g,
        fat_g,
        if let Some(ref conditions) = payload.medical_conditions {
            format!("- Medical conditions: {}", conditions.join(", "))
        } else {
            String::new()
        },
        if let Some(ref allergies) = payload.allergies {
            format!("- Allergies: {}", allergies.join(", "))
        } else {
            String::new()
        },
        if let Some(ref prefs) = payload.dietary_preferences {
            format!("- Dietary preferences: {:?}", prefs)
        } else {
            String::new()
        }
    );

    tracing::info!("Generating AI recommendations for user: {}", user_id);

    let ai_response = match state.gemini_service.get_text_response(&ai_prompt).await {
        Ok(response) => {
            tracing::info!("Successfully generated AI recommendations");
            response
        }
        Err(e) => {
            tracing::error!("Failed to get AI recommendations: {}", e);
            "Unable to generate AI recommendations at this time. Please try again later.".to_string()
        }
    };

    let recommended_foods = extract_recommended_foods(&ai_response);
    let foods_to_avoid = extract_foods_to_avoid(&ai_response);

    tracing::info!("Creating health profile struct for user: {}", user_id);

    let profile = HealthProfile {
        age: payload.age,
        gender: payload.gender,
        height_cm: payload.height_cm,
        weight_kg: payload.weight_kg,
        activity_level: payload.activity_level,
        goal: payload.goal,
        medical_conditions: payload.medical_conditions,
        blood_pressure: payload.blood_pressure,
        fasting_blood_sugar: payload.fasting_blood_sugar,
        allergies: payload.allergies,
        dietary_preferences: payload.dietary_preferences,
        bmi,
        bmi_category,
        bmr,
        tdee,
        daily_calories,
        daily_protein_g: protein_g,
        daily_carbs_g: carbs_g,
        daily_fat_g: fat_g,
        ai_recommendations: Some(ai_response),
        recommended_foods: Some(recommended_foods),
        foods_to_avoid: Some(foods_to_avoid),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let user_oid = ObjectId::parse_str(&user_id).map_err(|e| {
        tracing::error!("Invalid user ID: {}", e);
        AppError::BadRequest("Invalid user ID".to_string())
    })?;

    tracing::info!("Serializing profile to BSON for user: {}", user_id);

    let profile_bson = mongodb::bson::to_bson(&profile).map_err(|e| {
        tracing::error!("Failed to serialize health profile to BSON: {}", e);
        AppError::InternalError(anyhow::anyhow!("Failed to serialize health profile"))
    })?;

    tracing::info!("Updating user document in database for user: {}", user_id);

    let update =
        doc! {
        "$set": {
            "health_profile": profile_bson,
            "has_completed_health_survey": true,
            "updated_at": Utc::now(),
        }
    };

    state.db
        .collection::<User>("users")
        .update_one(doc! { "_id": user_oid }, update, None).await
        .map_err(|e| {
            tracing::error!("Database update failed: {}", e);
            AppError::InternalError(e.into())
        })?;

    tracing::info!("Successfully created health profile for user: {}", user_id);

    Ok((
        StatusCode::OK,
        Json(HealthProfileResponse {
            success: true,
            profile,
            message: "Health profile created successfully!".to_string(),
        }),
    ))
}

pub async fn get_profile(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>
) -> Result<impl IntoResponse, AppError> {
    let user_id = claims.sub;
    let user_oid = ObjectId::parse_str(&user_id).map_err(|_|
        AppError::BadRequest("Invalid user ID".to_string())
    )?;

    let user = state.db
        .collection::<User>("users")
        .find_one(doc! { "_id": user_oid }, None).await
        .map_err(|e| AppError::InternalError(e.into()))?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    match user.health_profile {
        Some(profile) => Ok((StatusCode::OK, Json(profile))),
        None =>
            Err(
                AppError::NotFound(
                    "Health profile not found. Please complete the health survey.".to_string()
                )
            ),
    }
}

fn extract_recommended_foods(ai_response: &str) -> Vec<String> {
    let mut foods = Vec::new();
    for line in ai_response.lines() {
        let trimmed = line.trim();
        if
            trimmed.starts_with('-') ||
            trimmed.starts_with('•') ||
            (trimmed.len() > 2 &&
                trimmed.chars().nth(0).unwrap().is_numeric() &&
                trimmed.chars().nth(1) == Some('.'))
        {
            if let Some(food) = trimmed.split_once(|c: char| (c == '-' || c == '•' || c == '.')) {
                let food_name = food.1.trim().to_string();
                if !food_name.is_empty() && food_name.len() < 100 {
                    foods.push(food_name);
                }
            }
        }
    }
    foods.into_iter().take(15).collect()
}

fn extract_foods_to_avoid(ai_response: &str) -> Vec<String> {
    let mut foods = Vec::new();
    let lower = ai_response.to_lowercase();

    if let Some(avoid_idx) = lower.find("avoid") {
        let avoid_section = &ai_response[avoid_idx..];
        for line in avoid_section.lines().take(20) {
            let trimmed = line.trim();
            if
                trimmed.starts_with('-') ||
                trimmed.starts_with('•') ||
                (trimmed.len() > 2 &&
                    trimmed.chars().nth(0).unwrap().is_numeric() &&
                    trimmed.chars().nth(1) == Some('.'))
            {
                if
                    let Some(food) = trimmed.split_once(
                        |c: char| (c == '-' || c == '•' || c == '.')
                    )
                {
                    let food_name = food.1.trim().to_string();
                    if !food_name.is_empty() && food_name.len() < 100 {
                        foods.push(food_name);
                    }
                }
            }
        }
    }

    foods.into_iter().take(10).collect()
}
