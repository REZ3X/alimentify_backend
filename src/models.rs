use chrono::{ DateTime, Utc };
use serde::{ Deserialize, Serialize };
use mongodb::bson::oid::ObjectId;

mod bson_datetime {
    use chrono::{ DateTime, Utc, TimeZone };
    use serde::{ self, Deserialize, Deserializer, Serializer };

    pub fn serialize<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        let s = date.to_rfc3339();
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
        where D: Deserializer<'de>
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum DateTimeFormat {
            String(String),
            BsonDateTime {
                #[serde(rename = "$date")]
                date: DateValue,
            },
        }

        #[derive(Deserialize)]
        #[serde(untagged)]
        enum DateValue {
            String(String),
            Timestamp {
                #[serde(rename = "$numberLong")]
                number_long: String,
            },
            Number(i64),
        }

        let value = DateTimeFormat::deserialize(deserializer)?;

        match value {
            DateTimeFormat::String(s) => {
                DateTime::parse_from_rfc3339(&s)
                    .map(|dt| dt.with_timezone(&Utc))
                    .or_else(|_| {
                        s.parse::<i64>()
                            .ok()
                            .and_then(|ts| Utc.timestamp_millis_opt(ts).single())
                            .ok_or_else(|| serde::de::Error::custom("Invalid datetime format"))
                    })
            }
            DateTimeFormat::BsonDateTime { date } => {
                match date {
                    DateValue::String(s) => {
                        DateTime::parse_from_rfc3339(&s)
                            .map(|dt| dt.with_timezone(&Utc))
                            .map_err(serde::de::Error::custom)
                    }
                    DateValue::Timestamp { number_long } => {
                        number_long
                            .parse::<i64>()
                            .ok()
                            .and_then(|ts| Utc.timestamp_millis_opt(ts).single())
                            .ok_or_else(|| serde::de::Error::custom("Invalid timestamp"))
                    }
                    DateValue::Number(ts) => {
                        Utc.timestamp_millis_opt(ts)
                            .single()
                            .ok_or_else(|| serde::de::Error::custom("Invalid timestamp"))
                    }
                }
            }
        }
    }
}

fn serialize_object_id_as_string<S>(
    id: &Option<mongodb::bson::oid::ObjectId>,
    serializer: S
) -> Result<S::Ok, S::Error>
    where S: serde::Serializer
{
    match id {
        Some(oid) => serializer.serialize_str(&oid.to_hex()),
        None => serializer.serialize_none(),
    }
}

mod bson_datetime_option {
    use chrono::{ DateTime, Utc, TimeZone };
    use serde::{ self, Deserialize, Deserializer, Serializer };

    pub fn serialize<S>(date: &Option<DateTime<Utc>>, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        match date {
            Some(d) => {
                let s = d.to_rfc3339();
                serializer.serialize_some(&s)
            }
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error>
        where D: Deserializer<'de>
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum DateTimeFormat {
            String(String),
            BsonDateTime {
                #[serde(rename = "$date")]
                date: DateValue,
            },
        }

        #[derive(Deserialize)]
        #[serde(untagged)]
        enum DateValue {
            String(String),
            Timestamp {
                #[serde(rename = "$numberLong")]
                number_long: String,
            },
            Number(i64),
        }

        let value: Option<DateTimeFormat> = Option::deserialize(deserializer)?;

        match value {
            None => Ok(None),
            Some(DateTimeFormat::String(s)) => {
                DateTime::parse_from_rfc3339(&s)
                    .map(|dt| Some(dt.with_timezone(&Utc)))
                    .or_else(|_| {
                        s.parse::<i64>()
                            .ok()
                            .and_then(|ts| Utc.timestamp_millis_opt(ts).single())
                            .map(Some)
                            .ok_or_else(|| serde::de::Error::custom("Invalid datetime format"))
                    })
            }
            Some(DateTimeFormat::BsonDateTime { date }) => {
                match date {
                    DateValue::String(s) => {
                        DateTime::parse_from_rfc3339(&s)
                            .map(|dt| Some(dt.with_timezone(&Utc)))
                            .map_err(serde::de::Error::custom)
                    }
                    DateValue::Timestamp { number_long } => {
                        number_long
                            .parse::<i64>()
                            .ok()
                            .and_then(|ts| Utc.timestamp_millis_opt(ts).single())
                            .map(Some)
                            .ok_or_else(|| serde::de::Error::custom("Invalid timestamp"))
                    }
                    DateValue::Number(ts) => {
                        Utc.timestamp_millis_opt(ts)
                            .single()
                            .map(Some)
                            .ok_or_else(|| serde::de::Error::custom("Invalid timestamp"))
                    }
                }
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub google_id: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub profile_image: Option<String>,
    pub username: String,
    pub name: String,
    pub gmail: String,
    pub email_verification_status: bool,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub email_verification_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default, with = "bson_datetime_option")]
    pub email_verified_at: Option<DateTime<Utc>>,
    #[serde(with = "bson_datetime")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "bson_datetime")]
    pub updated_at: DateTime<Utc>,

    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub health_profile: Option<HealthProfile>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub has_completed_health_survey: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: String,
    pub google_id: String,
    pub profile_image: Option<String>,
    pub username: String,
    pub name: String,
    pub gmail: String,
    pub email_verification_status: bool,
    pub email_verified_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub has_completed_health_survey: Option<bool>,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        UserResponse {
            id: user.id.map(|id| id.to_hex()).unwrap_or_default(),
            google_id: user.google_id,
            profile_image: user.profile_image,
            username: user.username,
            name: user.name,
            gmail: user.gmail,
            email_verification_status: user.email_verification_status,
            email_verified_at: user.email_verified_at,
            created_at: user.created_at,
            updated_at: user.updated_at,
            has_completed_health_survey: user.has_completed_health_survey,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GoogleUserInfo {
    pub id: String,
    pub email: String,
    pub verified_email: bool,
    pub name: String,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub picture: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserResponse,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub email: String,
    pub exp: i64,
    pub iat: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Session {
    pub user_id: String,
    pub email: String,
    #[serde(with = "bson_datetime")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "bson_datetime")]
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HealthProfile {
    pub age: i32,
    pub gender: Gender,
    pub height_cm: f64,
    pub weight_kg: f64,
    pub activity_level: ActivityLevel,
    pub goal: HealthGoal,

    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub medical_conditions: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub blood_pressure: Option<BloodPressure>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub fasting_blood_sugar: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub allergies: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub dietary_preferences: Option<Vec<DietaryPreference>>,

    pub bmi: f64,
    pub bmi_category: String,
    pub bmr: f64,
    pub tdee: f64,
    pub daily_calories: f64,
    pub daily_protein_g: f64,
    pub daily_carbs_g: f64,
    pub daily_fat_g: f64,

    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub ai_recommendations: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub recommended_foods: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub foods_to_avoid: Option<Vec<String>>,

    #[serde(with = "bson_datetime")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "bson_datetime")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Gender {
    Male,
    Female,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ActivityLevel {
    Sedentary,
    LightlyActive,
    ModeratelyActive,
    VeryActive,
    ExtraActive,
}

impl ActivityLevel {
    pub fn multiplier(&self) -> f64 {
        match self {
            ActivityLevel::Sedentary => 1.2,
            ActivityLevel::LightlyActive => 1.375,
            ActivityLevel::ModeratelyActive => 1.55,
            ActivityLevel::VeryActive => 1.725,
            ActivityLevel::ExtraActive => 1.9,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum HealthGoal {
    LoseWeight,
    MaintainWeight,
    GainWeight,
    BuildMuscle,
}

impl HealthGoal {
    pub fn calorie_adjustment(&self) -> f64 {
        match self {
            HealthGoal::LoseWeight => -500.0,
            HealthGoal::MaintainWeight => 0.0,
            HealthGoal::GainWeight => 300.0,
            HealthGoal::BuildMuscle => 500.0,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BloodPressure {
    pub systolic: i32,
    pub diastolic: i32,
    #[serde(with = "bson_datetime")]
    pub measured_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum DietaryPreference {
    Vegetarian,
    Vegan,
    Pescatarian,
    Halal,
    Kosher,
    GlutenFree,
    DairyFree,
    LowCarb,
    Keto,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MealLog {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub user_id: ObjectId,
    #[serde(with = "bson_datetime")]
    pub date: DateTime<Utc>,
    pub meal_type: MealType,
    pub food_name: String,
    pub calories: f64,
    pub protein_g: f64,
    pub carbs_g: f64,
    pub fat_g: f64,
    pub serving_size: Option<String>,
    pub notes: Option<String>,
    #[serde(with = "bson_datetime")]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum MealType {
    Breakfast,
    Lunch,
    Dinner,
    Snack,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DailyProgress {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub user_id: ObjectId,
    #[serde(with = "bson_datetime")]
    pub date: DateTime<Utc>,
    pub total_calories: f64,
    pub total_protein_g: f64,
    pub total_carbs_g: f64,
    pub total_fat_g: f64,
    pub water_ml: Option<f64>,
    pub weight_kg: Option<f64>,
    pub notes: Option<String>,
    #[serde(with = "bson_datetime")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "bson_datetime")]
    pub updated_at: DateTime<Utc>,
}

impl HealthProfile {
    pub fn calculate_bmi(weight_kg: f64, height_cm: f64) -> f64 {
        let height_m = height_cm / 100.0;
        weight_kg / (height_m * height_m)
    }

    pub fn bmi_category(bmi: f64) -> String {
        match bmi {
            bmi if bmi < 18.5 => "Underweight".to_string(),
            bmi if bmi < 25.0 => "Normal".to_string(),
            bmi if bmi < 30.0 => "Overweight".to_string(),
            _ => "Obese".to_string(),
        }
    }

    pub fn calculate_bmr(weight_kg: f64, height_cm: f64, age: i32, gender: &Gender) -> f64 {
        match gender {
            Gender::Male => 10.0 * weight_kg + 6.25 * height_cm - 5.0 * (age as f64) + 5.0,
            Gender::Female => 10.0 * weight_kg + 6.25 * height_cm - 5.0 * (age as f64) - 161.0,
        }
    }

    pub fn calculate_tdee(bmr: f64, activity_level: &ActivityLevel) -> f64 {
        bmr * activity_level.multiplier()
    }

    pub fn calculate_daily_calories(tdee: f64, goal: &HealthGoal) -> f64 {
        (tdee + goal.calorie_adjustment()).max(1200.0)
    }

    pub fn calculate_macros(daily_calories: f64, goal: &HealthGoal) -> (f64, f64, f64) {
        match goal {
            HealthGoal::LoseWeight => {
                let protein_g = (daily_calories * 0.3) / 4.0;
                let carbs_g = (daily_calories * 0.4) / 4.0;
                let fat_g = (daily_calories * 0.3) / 9.0;
                (protein_g, carbs_g, fat_g)
            }
            HealthGoal::BuildMuscle => {
                let protein_g = (daily_calories * 0.35) / 4.0;
                let carbs_g = (daily_calories * 0.4) / 4.0;
                let fat_g = (daily_calories * 0.25) / 9.0;
                (protein_g, carbs_g, fat_g)
            }
            _ => {
                let protein_g = (daily_calories * 0.25) / 4.0;
                let carbs_g = (daily_calories * 0.45) / 4.0;
                let fat_g = (daily_calories * 0.3) / 9.0;
                (protein_g, carbs_g, fat_g)
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ReportPeriod {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ReportStatus {
    Generated,
    Sent,
    Failed,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MealReport {
    #[serde(
        rename = "_id",
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_object_id_as_string"
    )]
    pub id: Option<ObjectId>,
    pub user_id: ObjectId,
    pub report_type: ReportPeriod,
    pub start_date: String,
    pub end_date: String,
    pub generated_at: DateTime<Utc>,
    pub status: ReportStatus,

    pub total_days: usize,
    pub days_logged: usize,
    pub total_meals: usize,
    pub avg_calories: f64,
    pub avg_protein_g: f64,
    pub avg_carbs_g: f64,
    pub avg_fat_g: f64,

    pub goal_type: String,
    pub goal_achieved: bool,
    pub calories_compliance_percent: f64,
    pub protein_compliance_percent: f64,
    pub carbs_compliance_percent: f64,
    pub fat_compliance_percent: f64,
    pub days_on_target: usize,

    pub starting_weight: Option<f64>,
    pub ending_weight: Option<f64>,
    pub weight_change: Option<f64>,
    pub target_weight: Option<f64>,
    pub weight_goal_achieved: Option<bool>,

    pub best_day_date: Option<String>,
    pub best_day_compliance: Option<f64>,
    pub streak_days: usize,
    pub notes: Option<String>,
}

// ==================== Chat Models ====================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatSession {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub user_id: ObjectId,
    pub title: String,
    #[serde(with = "bson_datetime")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "bson_datetime")]
    pub updated_at: DateTime<Utc>,
    pub message_count: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub session_id: ObjectId,
    pub user_id: ObjectId,
    pub role: MessageRole,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_results: Option<Vec<ToolResult>>,
    #[serde(with = "bson_datetime")]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolCall {
    pub tool_name: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolResult {
    pub tool_name: String,
    pub result: serde_json::Value,
    pub success: bool,
}
