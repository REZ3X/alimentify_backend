use anyhow::{ Context, Result };
use reqwest::Client;
use serde::{ Deserialize, Serialize };
use serde_json::Value;
use std::sync::Arc;

fn parse_flexible_number(value: &Value) -> f64 {
    match value {
        Value::Number(n) => n.as_f64().unwrap_or(0.0),
        Value::String(s) => {
            s.parse::<f64>().unwrap_or(0.0)
        }
        _ => 0.0,
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NinjaNutritionItem {
    pub name: String,
    #[serde(deserialize_with = "deserialize_flexible_number")]
    pub calories: f64,
    #[serde(deserialize_with = "deserialize_flexible_number")]
    pub serving_size_g: f64,
    #[serde(deserialize_with = "deserialize_flexible_number")]
    pub fat_total_g: f64,
    #[serde(deserialize_with = "deserialize_flexible_number")]
    pub fat_saturated_g: f64,
    #[serde(deserialize_with = "deserialize_flexible_number")]
    pub protein_g: f64,
    #[serde(deserialize_with = "deserialize_flexible_number")]
    pub sodium_mg: f64,
    #[serde(deserialize_with = "deserialize_flexible_number")]
    pub potassium_mg: f64,
    #[serde(deserialize_with = "deserialize_flexible_number")]
    pub cholesterol_mg: f64,
    #[serde(deserialize_with = "deserialize_flexible_number")]
    pub carbohydrates_total_g: f64,
    #[serde(deserialize_with = "deserialize_flexible_number")]
    pub fiber_g: f64,
    #[serde(deserialize_with = "deserialize_flexible_number")]
    pub sugar_g: f64,
}

fn deserialize_flexible_number<'de, D>(deserializer: D) -> Result<f64, D::Error>
    where D: serde::Deserializer<'de>
{
    let value = Value::deserialize(deserializer)?;
    Ok(parse_flexible_number(&value))
}

#[derive(Clone)]
pub struct NinjaService {
    client: Arc<Client>,
    api_key: String,
    base_url: String,
}

impl NinjaService {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Arc::new(Client::new()),
            api_key,
            base_url: "https://api.api-ninjas.com/v1".to_string(),
        }
    }

    pub async fn get_nutrition(&self, query: &str) -> Result<Vec<NinjaNutritionItem>> {
        let url = format!("{}/nutrition", self.base_url);

        tracing::debug!("Calling Ninja API with query: {}", query);

        let response = self.client
            .get(&url)
            .header("X-Api-Key", &self.api_key)
            .query(&[("query", query)])
            .send().await
            .context("Failed to send request to Ninja API")?;

        let status = response.status();
        tracing::debug!("Ninja API response status: {}", status);

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            tracing::error!("Ninja API error: {} - {}", status, error_text);
            anyhow::bail!("Ninja API error: {} - {}", status, error_text);
        }

        let response_text = response.text().await.context("Failed to get response text")?;

        tracing::debug!("Ninja API response body: {}", response_text);

        let result: Vec<NinjaNutritionItem> = serde_json
            ::from_str(&response_text)
            .context("Failed to parse Ninja API response")?;

        tracing::debug!("Successfully parsed {} nutrition items", result.len());

        Ok(result)
    }
}
