use anyhow::Result;
use mongodb::bson::{ doc, oid::ObjectId };
use chrono::{ Utc, TimeZone };
use serde::{ Deserialize, Serialize };
use serde_json::{ json, Value };
use std::sync::Arc;

use crate::{
    db::AppState,
    models::*,
    services::{ gemini_service::GeminiService, email_service::EmailService },
};

#[derive(Debug, Serialize)]
struct AgentRequest {
    user_context: UserContext,
    conversation_history: Vec<ChatMessageDto>,
    current_message: String,
}

#[derive(Debug, Serialize)]
struct UserContext {
    name: String,
    username: String,
    health_profile: Option<HealthProfile>,
    daily_targets: Option<DailyTargets>,
    has_completed_health_survey: bool,
}

#[derive(Debug, Serialize)]
struct DailyTargets {
    calories: f64,
    protein_g: f64,
    carbs_g: f64,
    fat_g: f64,
}

#[derive(Debug, Serialize)]
struct ChatMessageDto {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct AgentResponse {
    #[serde(default)]
    response: String,
    #[serde(default)]
    tool_calls: Vec<ToolCallRequest>,
}

#[derive(Debug, Deserialize, Clone)]
struct ToolCallRequest {
    tool_name: String,
    parameters: Value,
}

pub struct ChatAgentService {
    gemini: Arc<GeminiService>,
    email_service: Arc<EmailService>,
}

impl ChatAgentService {
    pub fn new(gemini: Arc<GeminiService>, email_service: Arc<EmailService>) -> Self {
        Self {
            gemini,
            email_service,
        }
    }

    pub async fn process_message(
        &self,
        state: &AppState,
        user_id: ObjectId,
        _session_id: ObjectId,
        message: &str,
        conversation_history: Vec<ChatMessage>
    ) -> Result<(String, Vec<ToolCall>, Vec<ToolResult>)> {
        let user = state.db
            .collection::<User>("users")
            .find_one(doc! { "_id": user_id }, None).await?
            .ok_or_else(|| anyhow::anyhow!("User not found"))?;

        let user_context = UserContext {
            name: user.name.clone(),
            username: user.username.clone(),
            health_profile: user.health_profile.clone(),
            daily_targets: user.health_profile.as_ref().map(|hp| DailyTargets {
                calories: hp.daily_calories,
                protein_g: hp.daily_protein_g,
                carbs_g: hp.daily_carbs_g,
                fat_g: hp.daily_fat_g,
            }),
            has_completed_health_survey: user.has_completed_health_survey.unwrap_or(false),
        };

        let history: Vec<ChatMessageDto> = conversation_history
            .iter()
            .map(|msg| ChatMessageDto {
                role: format!("{:?}", msg.role).to_lowercase(),
                content: msg.content.clone(),
            })
            .collect();

        let system_prompt = self.build_system_prompt(&user_context);

        let full_prompt = self.build_full_prompt(&system_prompt, &user_context, &history, message);

        tracing::info!("Sending message to Gemini AI agent");

        let ai_response = self.gemini.get_text_response(&full_prompt).await?;

        tracing::info!("Received response from Gemini AI agent");

        let (response_text, tool_calls, tool_results) = self.parse_and_execute_tools(
            state,
            user_id,
            &ai_response
        ).await?;

        Ok((response_text, tool_calls, tool_results))
    }

    fn build_system_prompt(&self, user_context: &UserContext) -> String {
        format!(
            r#"You are Alimentify AI, a personal nutrition and meal tracking assistant. You are helping {}.

YOUR CAPABILITIES (Tools you can use - ONLY for meal logging, stats, and reports):
1. LOG_MEAL - Log a meal with nutritional information
   Required parameters: meal_type (breakfast/lunch/dinner/snack), food_name, calories, protein_g, carbs_g, fat_g
   Optional parameters: serving_size, notes
2. GET_MEAL_LOGS - Retrieve past meal logs for a specific date or date range
3. GET_NUTRITION_STATS - Get nutrition statistics for a time period
   Parameters: period (daily/weekly/monthly/yearly) - defaults to weekly if not specified
   Returns: consumed and target values for calories, protein, carbs, fat
4. GET_HEALTH_PROFILE - Get user's health profile and goals
5. GENERATE_REPORT - Generate and optionally email nutrition reports
   Parameters: report_type (daily/weekly/monthly/yearly) - defaults to weekly, send_email (true/false)
   Returns: report_id and report_url for viewing the detailed report
6. CHECK_GOAL_PROGRESS - Check progress towards nutrition goals

USER PROFILE:
- Name: {}
- Username: {}
- Health Survey Completed: {}
{}

RESPONSE FORMAT:
When you need to use a tool, respond in this EXACT JSON format:
{{
  "response": "Your message to the user explaining what you're doing",
  "tool_calls": [
    {{
      "tool_name": "TOOL_NAME",
      "parameters": {{
        "param1": "value1",
        "param2": "value2"
      }}
    }}
  ]
}}

When just responding without tools, respond naturally in plain text.

IMPORTANT GUIDELINES:
1. Be friendly, conversational, and supportive - NEVER show raw JSON or technical data to users
2. Always consider the user's health profile when making suggestions
3. If the user hasn't completed their health survey, gently encourage them to do so
4. When analyzing meals, be constructive and provide helpful feedback in natural language
5. Proactively offer to help with meal logging, tracking, and goal setting
6. Use tools when appropriate to provide accurate, data-driven responses
7. Keep responses concise but informative
8. When user sends a meal image with analysis results, extract ALL nutrition values and use LOG_MEAL
9. For LOG_MEAL, you MUST provide all required numeric parameters: calories, protein_g, carbs_g, fat_g
10. Always verify user intent before executing actions like sending emails
11. When logging meals from images, parse the nutrition information from the message context
12. CRITICAL: Transform image analysis data into friendly conversation - describe the food, nutrition, and health insights naturally
13. Example: Instead of showing JSON, say "I can see that's a cheeseburger with about 550 calories, 25g protein, 45g carbs, and 30g fat. I'll log that for you!"
14. When GENERATE_REPORT returns a report_url, ALWAYS include a clickable markdown link in your response
    Example format: "Your weekly report is ready! [Click here to view it](http://localhost:3000/my/reports/ID)"
    CRITICAL: NO SPACES between ]( in markdown links - must be ](URL) not ] (URL)

CONVERSATION STYLE:
- Use natural language, avoid being overly formal
- Use emojis occasionally to be friendly (but not excessively)
- Ask clarifying questions when needed
- Provide context for your recommendations
- Celebrate user achievements and progress
"#,
            user_context.name,
            user_context.name,
            user_context.username,
            user_context.has_completed_health_survey,
            if let Some(ref profile) = user_context.health_profile {
                format!(
                    "\n- Goal: {:?}\n- Daily Calorie Target: {:.0} kcal\n- Activity Level: {:?}",
                    profile.goal,
                    profile.daily_calories,
                    profile.activity_level
                )
            } else {
                "\n- No health profile set yet".to_string()
            }
        )
    }

    fn build_full_prompt(
        &self,
        system_prompt: &str,
        _user_context: &UserContext,
        history: &[ChatMessageDto],
        current_message: &str
    ) -> String {
        let mut prompt = format!("{}\n\nCONVERSATION HISTORY:\n", system_prompt);

        let recent_history: Vec<&ChatMessageDto> = history.iter().rev().take(10).rev().collect();

        for msg in recent_history {
            prompt.push_str(&format!("{}: {}\n", msg.role.to_uppercase(), msg.content));
        }

        prompt.push_str(&format!("\nUSER: {}\n\nASSISTANT:", current_message));

        prompt
    }

    async fn parse_and_execute_tools(
        &self,
        state: &AppState,
        user_id: ObjectId,
        ai_response: &str
    ) -> Result<(String, Vec<ToolCall>, Vec<ToolResult>)> {
        if let Ok(agent_response) = serde_json::from_str::<AgentResponse>(ai_response) {
            if !agent_response.tool_calls.is_empty() {
                let mut tool_calls = Vec::new();
                let mut tool_results = Vec::new();

                for tool_call in agent_response.tool_calls {
                    tracing::info!("Executing tool: {}", tool_call.tool_name);

                    let result = self.execute_tool(state, user_id, &tool_call).await;

                    let (success, result_value) = match result {
                        Ok(value) => (true, value),
                        Err(e) => {
                            tracing::error!("Tool execution failed: {}", e);
                            (false, json!({ "error": e.to_string() }))
                        }
                    };

                    tool_calls.push(ToolCall {
                        tool_name: tool_call.tool_name.clone(),
                        parameters: tool_call.parameters.clone(),
                    });

                    tool_results.push(ToolResult {
                        tool_name: tool_call.tool_name.clone(),
                        result: result_value.clone(),
                        success,
                    });
                }

                let tool_results_text = tool_results
                    .iter()
                    .map(|tr|
                        format!(
                            "Tool: {}\nResult: {}",
                            tr.tool_name,
                            serde_json::to_string_pretty(&tr.result).unwrap_or_default()
                        )
                    )
                    .collect::<Vec<String>>()
                    .join("\n\n");

                let follow_up_prompt = format!(
                    "{}\n\nTOOL RESULTS:\n{}\n\nNow provide a natural, conversational response to the user using the tool results above. Format the data in a friendly, easy-to-read way.",
                    agent_response.response,
                    tool_results_text
                );

                let final_response = self.gemini.get_text_response(&follow_up_prompt).await?;

                return Ok((final_response, tool_calls, tool_results));
            }

            return Ok((agent_response.response, vec![], vec![]));
        }

        Ok((ai_response.to_string(), vec![], vec![]))
    }

    async fn execute_tool(
        &self,
        state: &AppState,
        user_id: ObjectId,
        tool_call: &ToolCallRequest
    ) -> Result<Value> {
        match tool_call.tool_name.as_str() {
            "LOG_MEAL" => self.tool_log_meal(state, user_id, &tool_call.parameters).await,
            "GET_MEAL_LOGS" => self.tool_get_meal_logs(state, user_id, &tool_call.parameters).await,
            "GET_NUTRITION_STATS" | "GET_DAILY_STATS" =>
                self.tool_get_nutrition_stats(state, user_id, &tool_call.parameters).await,
            "GET_HEALTH_PROFILE" => self.tool_get_health_profile(state, user_id).await,
            "GENERATE_REPORT" =>
                self.tool_generate_report(state, user_id, &tool_call.parameters).await,
            "CHECK_GOAL_PROGRESS" => self.tool_check_goal_progress(state, user_id).await,
            _ => Err(anyhow::anyhow!("Unknown tool: {}", tool_call.tool_name)),
        }
    }

    async fn tool_log_meal(
        &self,
        state: &AppState,
        user_id: ObjectId,
        params: &Value
    ) -> Result<Value> {
        let meal_type_str = params["meal_type"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing meal_type"))?;

        let meal_type = match meal_type_str.to_lowercase().as_str() {
            "breakfast" => MealType::Breakfast,
            "lunch" => MealType::Lunch,
            "dinner" => MealType::Dinner,
            "snack" => MealType::Snack,
            _ => {
                return Err(anyhow::anyhow!("Invalid meal_type"));
            }
        };

        let get_numeric = |key: &str| -> f64 {
            params[key]
                .as_f64()
                .or_else(|| params[key].as_i64().map(|v| v as f64))
                .or_else(|| params[key].as_str().and_then(|s| s.parse::<f64>().ok()))
                .unwrap_or(0.0)
        };

        let calories = get_numeric("calories");
        let protein_g = get_numeric("protein_g");
        let carbs_g = get_numeric("carbs_g");
        let fat_g = get_numeric("fat_g");

        if calories == 0.0 {
            return Err(anyhow::anyhow!("calories must be greater than 0"));
        }

        let meal_log = MealLog {
            id: None,
            user_id,
            date: Utc::now(),
            meal_type,
            food_name: params["food_name"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing food_name"))?
                .to_string(),
            calories,
            protein_g,
            carbs_g,
            fat_g,
            serving_size: params["serving_size"].as_str().map(|s| s.to_string()),
            notes: params["notes"].as_str().map(|s| s.to_string()),
            created_at: Utc::now(),
        };

        let result = state.db.collection::<MealLog>("meal_logs").insert_one(&meal_log, None).await?;

        Ok(
            json!({
            "success": true,
            "meal_id": result.inserted_id.as_object_id().unwrap().to_hex(),
            "message": "Meal logged successfully"
        })
        )
    }

    async fn tool_get_meal_logs(
        &self,
        state: &AppState,
        user_id: ObjectId,
        params: &Value
    ) -> Result<Value> {
        use futures::stream::TryStreamExt;

        let date_str = params["date"].as_str();

        let query = if let Some(date_str) = date_str {
            let date = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d")?;
            let start = chrono::Utc.from_utc_datetime(&date.and_hms_opt(0, 0, 0).unwrap());
            let end = chrono::Utc.from_utc_datetime(&date.and_hms_opt(23, 59, 59).unwrap());

            let start_bson = mongodb::bson::DateTime::from_chrono(start);
            let end_bson = mongodb::bson::DateTime::from_chrono(end);

            doc! {
                "user_id": user_id,
                "date": {
                    "$gte": start_bson,
                    "$lte": end_bson,
                }
            }
        } else {
            let today = Utc::now().date_naive();
            let start = chrono::Utc.from_utc_datetime(&today.and_hms_opt(0, 0, 0).unwrap());
            let end = chrono::Utc.from_utc_datetime(&today.and_hms_opt(23, 59, 59).unwrap());

            let start_bson = mongodb::bson::DateTime::from_chrono(start);
            let end_bson = mongodb::bson::DateTime::from_chrono(end);

            doc! {
                "user_id": user_id,
                "date": {
                    "$gte": start_bson,
                    "$lte": end_bson,
                }
            }
        };

        let mut cursor = state.db.collection::<MealLog>("meal_logs").find(query, None).await?;

        let mut meals = Vec::new();
        while let Some(meal) = cursor.try_next().await? {
            meals.push(
                json!({
                "id": meal.id.unwrap().to_hex(),
                "meal_type": format!("{:?}", meal.meal_type),
                "food_name": meal.food_name,
                "calories": meal.calories,
                "protein_g": meal.protein_g,
                "carbs_g": meal.carbs_g,
                "fat_g": meal.fat_g,
                "serving_size": meal.serving_size,
                "notes": meal.notes,
                "date": meal.date.to_rfc3339(),
            })
            );
        }

        Ok(
            json!({
            "success": true,
            "meals": meals,
            "count": meals.len()
        })
        )
    }

    async fn tool_get_nutrition_stats(
        &self,
        state: &AppState,
        user_id: ObjectId,
        params: &Value
    ) -> Result<Value> {
        use futures::stream::TryStreamExt;

        let period = params["period"].as_str().unwrap_or("weekly");

        tracing::info!("GET_NUTRITION_STATS: Fetching {} stats", period);

        let today = Utc::now();
        let (start_date, end_date) = match period {
            "daily" => {
                let start_of_day = chrono::Utc.from_utc_datetime(
                    &today.date_naive().and_hms_opt(0, 0, 0).unwrap()
                );
                (start_of_day, start_of_day + chrono::Duration::days(1))
            }
            "monthly" => {
                let start = today - chrono::Duration::days(29);
                let start_of_period = chrono::Utc.from_utc_datetime(
                    &start.date_naive().and_hms_opt(0, 0, 0).unwrap()
                );
                (start_of_period, today)
            }
            "yearly" => {
                let start = today - chrono::Duration::days(364);
                let start_of_period = chrono::Utc.from_utc_datetime(
                    &start.date_naive().and_hms_opt(0, 0, 0).unwrap()
                );
                (start_of_period, today)
            }
            _ => {
                let start = today - chrono::Duration::days(6);
                let start_of_period = chrono::Utc.from_utc_datetime(
                    &start.date_naive().and_hms_opt(0, 0, 0).unwrap()
                );
                (start_of_period, today)
            }
        };

        let start_bson = mongodb::bson::DateTime::from_chrono(start_date);
        let end_bson = mongodb::bson::DateTime::from_chrono(end_date);

        tracing::info!("GET_NUTRITION_STATS: Querying meals from {} to {}", start_date, end_date);

        let all_meals_cursor = state.db
            .collection::<MealLog>("meal_logs")
            .find(doc! { "user_id": user_id }, None).await?;

        let all_meals: Vec<MealLog> = all_meals_cursor.try_collect().await?;
        tracing::info!("GET_NUTRITION_STATS: Total meals in DB for user: {}", all_meals.len());

        for (i, meal) in all_meals.iter().take(3).enumerate() {
            tracing::info!(
                "GET_NUTRITION_STATS: Sample meal {}: {} at {}",
                i + 1,
                meal.food_name,
                meal.date
            );
        }

        let mut cursor = state.db
            .collection::<MealLog>("meal_logs")
            .find(
                doc! {
                    "user_id": user_id,
                    "date": {
                        "$gte": start_bson,
                        "$lt": end_bson,
                    }
                },
                None
            ).await?;

        let mut meals_in_range: Vec<MealLog> = Vec::new();
        while let Some(meal) = cursor.try_next().await? {
            meals_in_range.push(meal);
        }

        tracing::info!("GET_NUTRITION_STATS: Found {} meals with date query", meals_in_range.len());

        if meals_in_range.is_empty() && !all_meals.is_empty() {
            tracing::warn!("GET_NUTRITION_STATS: Date query returned 0, filtering manually");
            meals_in_range = all_meals
                .into_iter()
                .filter(|meal| {
                    let meal_date = meal.date;
                    let in_range = meal_date >= start_date && meal_date < end_date;
                    if in_range {
                        tracing::info!(
                            "GET_NUTRITION_STATS: Manual filter matched: {} at {}",
                            meal.food_name,
                            meal_date
                        );
                    }
                    in_range
                })
                .collect();
            tracing::info!("GET_NUTRITION_STATS: Manually filtered {} meals", meals_in_range.len());
        }

        let mut total_calories = 0.0;
        let mut total_protein = 0.0;
        let mut total_carbs = 0.0;
        let mut total_fat = 0.0;
        let meal_count = meals_in_range.len();

        for meal in meals_in_range {
            tracing::info!(
                "GET_NUTRITION_STATS: Including meal - {} ({}cal)",
                meal.food_name,
                meal.calories
            );
            total_calories += meal.calories;
            total_protein += meal.protein_g;
            total_carbs += meal.carbs_g;
            total_fat += meal.fat_g;
        }

        tracing::info!(
            "GET_NUTRITION_STATS: Totals - {} meals, calories: {}, protein: {}, carbs: {}, fat: {}",
            meal_count,
            total_calories,
            total_protein,
            total_carbs,
            total_fat
        );

        let user = state.db
            .collection::<User>("users")
            .find_one(doc! { "_id": user_id }, None).await?
            .ok_or_else(|| anyhow::anyhow!("User not found"))?;

        let (target_calories, target_protein, target_carbs, target_fat) = if
            let Some(ref profile) = user.health_profile
        {
            (
                profile.daily_calories,
                profile.daily_protein_g,
                profile.daily_carbs_g,
                profile.daily_fat_g,
            )
        } else {
            (2000.0, 50.0, 250.0, 70.0) 
        };

        Ok(
            json!({
            "success": true,
            "current": {
                "calories": total_calories,
                "protein_g": total_protein,
                "carbs_g": total_carbs,
                "fat_g": total_fat
            },
            "targets": {
                "calories": target_calories,
                "protein_g": target_protein,
                "carbs_g": target_carbs,
                "fat_g": target_fat
            },
            "remaining": {
                "calories": target_calories - total_calories,
                "protein_g": target_protein - total_protein,
                "carbs_g": target_carbs - total_carbs,
                "fat_g": target_fat - total_fat
            },
            "percentage": {
                "calories": (total_calories / target_calories * 100.0).min(100.0),
                "protein": (total_protein / target_protein * 100.0).min(100.0),
                "carbs": (total_carbs / target_carbs * 100.0).min(100.0),
                "fat": (total_fat / target_fat * 100.0).min(100.0)
            }
        })
        )
    }

    async fn tool_get_health_profile(&self, state: &AppState, user_id: ObjectId) -> Result<Value> {
        let user = state.db
            .collection::<User>("users")
            .find_one(doc! { "_id": user_id }, None).await?
            .ok_or_else(|| anyhow::anyhow!("User not found"))?;

        if let Some(profile) = user.health_profile {
            Ok(
                json!({
                "success": true,
                "profile": {
                    "age": profile.age,
                    "gender": format!("{:?}", profile.gender),
                    "height_cm": profile.height_cm,
                    "weight_kg": profile.weight_kg,
                    "activity_level": format!("{:?}", profile.activity_level),
                    "goal": format!("{:?}", profile.goal),
                    "bmi": profile.bmi,
                    "bmi_category": profile.bmi_category,
                    "daily_calories": profile.daily_calories,
                    "daily_protein_g": profile.daily_protein_g,
                    "daily_carbs_g": profile.daily_carbs_g,
                    "daily_fat_g": profile.daily_fat_g,
                    "dietary_preferences": profile.dietary_preferences,
                    "allergies": profile.allergies,
                }
            })
            )
        } else {
            Ok(
                json!({
                "success": false,
                "message": "User has not completed health survey yet"
            })
            )
        }
    }

    async fn tool_generate_report(
        &self,
        state: &AppState,
        user_id: ObjectId,
        params: &Value
    ) -> Result<Value> {
        use crate::models::{ MealReport, ReportPeriod, ReportStatus, MealLog };
        use crate::services::email_service::EmailService;
        use chrono::{ Utc, Duration, Timelike };
        use futures::stream::TryStreamExt;

        let report_type_str = params["report_type"].as_str().unwrap_or("weekly");
        let send_email = params["send_email"].as_bool().unwrap_or(false);

        let user = state.db
            .collection::<User>("users")
            .find_one(doc! { "_id": user_id }, None).await?
            .ok_or_else(|| anyhow::anyhow!("User not found"))?;

        let report_type = match report_type_str.to_lowercase().as_str() {
            "daily" => ReportPeriod::Daily,
            "weekly" => ReportPeriod::Weekly,
            "monthly" => ReportPeriod::Monthly,
            "yearly" => ReportPeriod::Yearly,
            _ => ReportPeriod::Weekly,
        };

        let now = Utc::now();
        let (start_date, end_date) = match report_type {
            ReportPeriod::Daily => {
                let today = now.date_naive();
                (today, today)
            }
            ReportPeriod::Weekly => {
                let today = now.date_naive();
                let start = today - Duration::days(7);
                (start, today)
            }
            ReportPeriod::Monthly => {
                let today = now.date_naive();
                let start = today - Duration::days(30);
                (start, today)
            }
            ReportPeriod::Yearly => {
                let today = now.date_naive();
                let start = today - Duration::days(365);
                (start, today)
            }
        };

        let start_datetime = chrono::TimeZone::from_utc_datetime(
            &chrono::Utc,
            &start_date.and_hms_opt(0, 0, 0).unwrap()
        );
        let end_datetime = chrono::TimeZone::from_utc_datetime(
            &chrono::Utc,
            &end_date.and_hms_opt(23, 59, 59).unwrap()
        );

        let start_bson = mongodb::bson::DateTime::from_chrono(start_datetime);
        let end_bson = mongodb::bson::DateTime::from_chrono(end_datetime);

        tracing::info!(
            "Chat Agent: Generating {} report for user {} from {} to {}",
            report_type_str,
            user_id,
            start_datetime,
            end_datetime
        );

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
                None
            ).await?;

        let mut meals: Vec<MealLog> = Vec::new();
        while let Some(meal) = cursor.try_next().await? {
            meals.push(meal);
        }

        tracing::info!("Chat Agent: Found {} meals with BSON date query", meals.len());

        if meals.is_empty() {
            tracing::warn!("Chat Agent: No meals found with BSON query, trying manual filtering");
            let all_meals_cursor = state.db
                .collection::<MealLog>("meal_logs")
                .find(doc! { "user_id": user_id }, None).await?;

            let all_meals: Vec<MealLog> = all_meals_cursor.try_collect().await?;

            meals = all_meals
                .into_iter()
                .filter(|meal| {
                    let meal_date = meal.date;
                    meal_date >= start_datetime && meal_date <= end_datetime
                })
                .collect();

            tracing::info!("Chat Agent: Manually filtered {} meals", meals.len());
        }

        let total_days = ((end_date - start_date).num_days() as usize) + 1;
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
        let avg_calories = if days_logged > 0 {
            total_calories / (days_logged as f64)
        } else {
            0.0
        };
        let avg_protein = if days_logged > 0 { total_protein / (days_logged as f64) } else { 0.0 };
        let avg_carbs = if days_logged > 0 { total_carbs / (days_logged as f64) } else { 0.0 };
        let avg_fat = if days_logged > 0 { total_fat / (days_logged as f64) } else { 0.0 };

        let (target_calories, target_protein, target_carbs, target_fat, goal_type) = if
            let Some(profile) = &user.health_profile
        {
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
            ((avg_calories / target_calories) * 100.0).min(100.0)
        } else {
            0.0
        };
        let protein_compliance = if target_protein > 0.0 {
            ((avg_protein / target_protein) * 100.0).min(100.0)
        } else {
            0.0
        };
        let carbs_compliance = if target_carbs > 0.0 {
            ((avg_carbs / target_carbs) * 100.0).min(100.0)
        } else {
            0.0
        };
        let fat_compliance = if target_fat > 0.0 {
            ((avg_fat / target_fat) * 100.0).min(100.0)
        } else {
            0.0
        };

        let days_on_target = days_with_meals
            .iter()
            .filter(|date| {
                let day_meals: Vec<&MealLog> = meals
                    .iter()
                    .filter(|m| m.date.date_naive() == **date)
                    .collect();
                let day_calories: f64 = day_meals
                    .iter()
                    .map(|m| m.calories)
                    .sum();
                let diff = (day_calories - target_calories).abs();
                diff / target_calories <= 0.1
            })
            .count();

        let avg_compliance =
            (calories_compliance + protein_compliance + carbs_compliance + fat_compliance) / 4.0;
        let goal_achieved =
            avg_compliance >= 80.0 && (days_logged as f64) / (total_days as f64) >= 0.7;

        let mut best_day_date = None;
        let mut best_day_compliance = 0.0;
        for date in &days_with_meals {
            let day_meals: Vec<&MealLog> = meals
                .iter()
                .filter(|m| m.date.date_naive() == *date)
                .collect();
            let day_calories: f64 = day_meals
                .iter()
                .map(|m| m.calories)
                .sum();
            let day_protein: f64 = day_meals
                .iter()
                .map(|m| m.protein_g)
                .sum();
            let day_carbs: f64 = day_meals
                .iter()
                .map(|m| m.carbs_g)
                .sum();
            let day_fat: f64 = day_meals
                .iter()
                .map(|m| m.fat_g)
                .sum();

            let day_cal_comp = ((day_calories / target_calories) * 100.0).min(100.0);
            let day_prot_comp = ((day_protein / target_protein) * 100.0).min(100.0);
            let day_carb_comp = ((day_carbs / target_carbs) * 100.0).min(100.0);
            let day_fat_comp = ((day_fat / target_fat) * 100.0).min(100.0);
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
            start_date: start_date.format("%Y-%m-%d").to_string(),
            end_date: end_date.format("%Y-%m-%d").to_string(),
            generated_at: Utc::now(),
            status: if send_email {
                ReportStatus::Sent
            } else {
                ReportStatus::Generated
            },
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
            best_day_compliance: if best_day_compliance > 0.0 {
                Some(best_day_compliance)
            } else {
                None
            },
            streak_days: streak,
            notes: None,
        };

        let result = state.db
            .collection::<MealReport>("meal_reports")
            .insert_one(&report, None).await?;

        let report_id = result.inserted_id.as_object_id().unwrap();
        let mut saved_report = report.clone();
        saved_report.id = Some(report_id);

        if send_email {
            let email_service = EmailService::new(
                state.config.brevo.smtp_host.clone(),
                state.config.brevo.smtp_port,
                state.config.brevo.smtp_user.clone(),
                state.config.brevo.smtp_pass.clone(),
                state.config.brevo.from_email.clone(),
                state.config.brevo.from_name.clone()
            );

            if let Err(e) = email_service.send_report_email(&user, &saved_report).await {
                tracing::error!("Chat Agent: Failed to send report email: {}", e);
                state.db
                    .collection::<MealReport>("meal_reports")
                    .update_one(
                        doc! { "_id": report_id },
                        doc! { "$set": { "status": "Failed" } },
                        None
                    ).await?;
            } else {
                tracing::info!("Chat Agent: Report email sent successfully");
            }
        }

        tracing::info!("Chat Agent: Report generated successfully with ID: {}", report_id);

        let report_url = format!(
            "{}/my/reports/{}",
            state.config.server.frontend_url,
            report_id.to_hex()
        );

        Ok(
            json!({
            "success": true,
            "report_id": report_id.to_hex(),
            "report_url": report_url,
            "report_type": report_type_str,
            "start_date": start_date.format("%Y-%m-%d").to_string(),
            "end_date": end_date.format("%Y-%m-%d").to_string(),
            "days_logged": days_logged,
            "total_days": total_days,
            "goal_achieved": goal_achieved,
            "avg_compliance": format!("{:.1}%", avg_compliance),
            "email_sent": send_email,
            "message": if send_email {
                format!("Your {} report has been generated and sent to your email!", report_type_str)
            } else {
                format!("Your {} report has been generated successfully!", report_type_str)
            }
        })
        )
    }

    async fn tool_check_goal_progress(&self, state: &AppState, user_id: ObjectId) -> Result<Value> {
        let stats = self.tool_get_nutrition_stats(
            state,
            user_id,
            &json!({"period": "daily"})
        ).await?;

        let user = state.db
            .collection::<User>("users")
            .find_one(doc! { "_id": user_id }, None).await?
            .ok_or_else(|| anyhow::anyhow!("User not found"))?;

        let goal_description = if let Some(ref profile) = user.health_profile {
            format!("{:?}", profile.goal)
        } else {
            "No goal set".to_string()
        };

        Ok(
            json!({
            "success": true,
            "goal": goal_description,
            "daily_progress": stats,
            "message": "Goal progress retrieved successfully"
        })
        )
    }

    pub async fn generate_chat_title(&self, first_message: &str) -> Result<String> {
        let prompt =
            format!(r#"Generate a short, concise title (maximum 5 words) for a chat conversation that starts with this message:

"{}"

Return ONLY the title, nothing else. Make it descriptive but brief."#, first_message);

        let title = self.gemini.get_text_response(&prompt).await?;

        let clean_title = title.trim().trim_matches('"').chars().take(50).collect::<String>();

        Ok(if clean_title.is_empty() { "New Chat".to_string() } else { clean_title })
    }
}
