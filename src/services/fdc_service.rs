use anyhow::{ Context, Result };
use reqwest::Client;
use serde::{ Deserialize, Serialize };
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize)]
pub struct FoodSearchResult {
    #[serde(rename = "totalHits")]
    pub total_hits: i32,
    #[serde(rename = "currentPage")]
    pub current_page: i32,
    #[serde(rename = "totalPages")]
    pub total_pages: i32,
    #[serde(rename = "foods")]
    pub foods: Vec<FoodItem>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FoodItem {
    #[serde(rename = "fdcId")]
    pub fdc_id: i32,
    pub description: String,
    #[serde(rename = "dataType")]
    pub data_type: Option<String>,
    #[serde(rename = "gtinUpc")]
    pub gtin_upc: Option<String>,
    #[serde(rename = "brandOwner")]
    pub brand_owner: Option<String>,
    #[serde(rename = "brandName")]
    pub brand_name: Option<String>,
    pub ingredients: Option<String>,
    #[serde(rename = "foodNutrients")]
    pub food_nutrients: Option<Vec<FoodNutrient>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FoodNutrient {
    #[serde(rename = "nutrientId")]
    pub nutrient_id: i32,
    #[serde(rename = "nutrientName")]
    pub nutrient_name: String,
    #[serde(rename = "nutrientNumber")]
    pub nutrient_number: Option<String>,
    #[serde(rename = "unitName")]
    pub unit_name: String,
    pub value: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FoodDetails {
    #[serde(rename = "fdcId")]
    pub fdc_id: i32,
    pub description: String,
    #[serde(rename = "dataType")]
    pub data_type: String,
    #[serde(rename = "foodClass")]
    pub food_class: Option<String>,
    #[serde(rename = "foodCategory")]
    pub food_category: Option<FoodCategory>,
    #[serde(rename = "foodNutrients")]
    pub food_nutrients: Vec<FoodNutrientDetail>,
    #[serde(rename = "foodPortions")]
    pub food_portions: Option<Vec<FoodPortion>>,
    #[serde(rename = "brandOwner")]
    pub brand_owner: Option<String>,
    #[serde(rename = "brandName")]
    pub brand_name: Option<String>,
    #[serde(rename = "gtinUpc")]
    pub gtin_upc: Option<String>,
    pub ingredients: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FoodCategory {
    pub id: i32,
    pub code: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FoodNutrientDetail {
    pub id: Option<i32>,
    pub amount: Option<f64>,
    pub nutrient: Nutrient,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Nutrient {
    pub id: i32,
    pub number: String,
    pub name: String,
    #[serde(rename = "unitName")]
    pub unit_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FoodPortion {
    pub id: Option<i32>,
    pub amount: Option<f64>,
    pub modifier: Option<String>,
    #[serde(rename = "gramWeight")]
    pub gram_weight: Option<f64>,
    #[serde(rename = "sequenceNumber")]
    pub sequence_number: Option<i32>,
}

#[derive(Clone)]
pub struct FdcService {
    client: Arc<Client>,
    api_key: String,
    base_url: String,
}

impl FdcService {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Arc::new(Client::new()),
            api_key,
            base_url: "https://api.nal.usda.gov/fdc/v1".to_string(),
        }
    }

    pub async fn search_foods(
        &self,
        query: &str,
        page_number: Option<i32>,
        page_size: Option<i32>,
        data_type: Option<Vec<String>>
    ) -> Result<FoodSearchResult> {
        let url = format!("{}/foods/search", self.base_url);

        let mut params = vec![("api_key", self.api_key.clone()), ("query", query.to_string())];

        if let Some(page) = page_number {
            params.push(("pageNumber", page.to_string()));
        }

        if let Some(size) = page_size {
            params.push(("pageSize", size.to_string()));
        }

        if let Some(types) = data_type {
            let types_str = types.join(",");
            params.push(("dataType", types_str));
        }

        let response = self.client
            .get(&url)
            .query(&params)
            .send().await
            .context("Failed to send request to FDC API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("FDC API error: {} - {}", status, error_text);
        }

        let result = response
            .json::<FoodSearchResult>().await
            .context("Failed to parse FDC API response")?;

        Ok(result)
    }

    pub async fn get_food_details(&self, fdc_id: i32) -> Result<FoodDetails> {
        let url = format!("{}/food/{}", self.base_url, fdc_id);

        let response = self.client
            .get(&url)
            .query(&[("api_key", &self.api_key)])
            .send().await
            .context("Failed to send request to FDC API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("FDC API error: {} - {}", status, error_text);
        }

        let result = response
            .json::<FoodDetails>().await
            .context("Failed to parse FDC API response")?;

        Ok(result)
    }

    pub async fn get_foods(&self, fdc_ids: Vec<i32>) -> Result<Vec<FoodDetails>> {
        let url = format!("{}/foods", self.base_url);

        let response = self.client
            .post(&url)
            .query(&[("api_key", &self.api_key)])
            .json(&fdc_ids)
            .send().await
            .context("Failed to send request to FDC API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("FDC API error: {} - {}", status, error_text);
        }

        let result = response
            .json::<Vec<FoodDetails>>().await
            .context("Failed to parse FDC API response")?;

        Ok(result)
    }
}
