use anyhow::{ Context, Result };
use reqwest::Client;
use serde::{ Deserialize, Serialize };
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize)]
pub struct MealsResponse {
    pub meals: Option<Vec<Meal>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Meal {
    #[serde(rename = "idMeal")]
    pub id_meal: String,
    #[serde(rename = "strMeal")]
    pub str_meal: String,
    #[serde(rename = "strDrinkAlternate")]
    pub str_drink_alternate: Option<String>,
    #[serde(rename = "strCategory")]
    pub str_category: Option<String>,
    #[serde(rename = "strArea")]
    pub str_area: Option<String>,
    #[serde(rename = "strInstructions")]
    pub str_instructions: Option<String>,
    #[serde(rename = "strMealThumb")]
    pub str_meal_thumb: Option<String>,
    #[serde(rename = "strTags")]
    pub str_tags: Option<String>,
    #[serde(rename = "strYoutube")]
    pub str_youtube: Option<String>,

    #[serde(rename = "strIngredient1")]
    pub str_ingredient1: Option<String>,
    #[serde(rename = "strIngredient2")]
    pub str_ingredient2: Option<String>,
    #[serde(rename = "strIngredient3")]
    pub str_ingredient3: Option<String>,
    #[serde(rename = "strIngredient4")]
    pub str_ingredient4: Option<String>,
    #[serde(rename = "strIngredient5")]
    pub str_ingredient5: Option<String>,
    #[serde(rename = "strIngredient6")]
    pub str_ingredient6: Option<String>,
    #[serde(rename = "strIngredient7")]
    pub str_ingredient7: Option<String>,
    #[serde(rename = "strIngredient8")]
    pub str_ingredient8: Option<String>,
    #[serde(rename = "strIngredient9")]
    pub str_ingredient9: Option<String>,
    #[serde(rename = "strIngredient10")]
    pub str_ingredient10: Option<String>,
    #[serde(rename = "strIngredient11")]
    pub str_ingredient11: Option<String>,
    #[serde(rename = "strIngredient12")]
    pub str_ingredient12: Option<String>,
    #[serde(rename = "strIngredient13")]
    pub str_ingredient13: Option<String>,
    #[serde(rename = "strIngredient14")]
    pub str_ingredient14: Option<String>,
    #[serde(rename = "strIngredient15")]
    pub str_ingredient15: Option<String>,
    #[serde(rename = "strIngredient16")]
    pub str_ingredient16: Option<String>,
    #[serde(rename = "strIngredient17")]
    pub str_ingredient17: Option<String>,
    #[serde(rename = "strIngredient18")]
    pub str_ingredient18: Option<String>,
    #[serde(rename = "strIngredient19")]
    pub str_ingredient19: Option<String>,
    #[serde(rename = "strIngredient20")]
    pub str_ingredient20: Option<String>,

    #[serde(rename = "strMeasure1")]
    pub str_measure1: Option<String>,
    #[serde(rename = "strMeasure2")]
    pub str_measure2: Option<String>,
    #[serde(rename = "strMeasure3")]
    pub str_measure3: Option<String>,
    #[serde(rename = "strMeasure4")]
    pub str_measure4: Option<String>,
    #[serde(rename = "strMeasure5")]
    pub str_measure5: Option<String>,
    #[serde(rename = "strMeasure6")]
    pub str_measure6: Option<String>,
    #[serde(rename = "strMeasure7")]
    pub str_measure7: Option<String>,
    #[serde(rename = "strMeasure8")]
    pub str_measure8: Option<String>,
    #[serde(rename = "strMeasure9")]
    pub str_measure9: Option<String>,
    #[serde(rename = "strMeasure10")]
    pub str_measure10: Option<String>,
    #[serde(rename = "strMeasure11")]
    pub str_measure11: Option<String>,
    #[serde(rename = "strMeasure12")]
    pub str_measure12: Option<String>,
    #[serde(rename = "strMeasure13")]
    pub str_measure13: Option<String>,
    #[serde(rename = "strMeasure14")]
    pub str_measure14: Option<String>,
    #[serde(rename = "strMeasure15")]
    pub str_measure15: Option<String>,
    #[serde(rename = "strMeasure16")]
    pub str_measure16: Option<String>,
    #[serde(rename = "strMeasure17")]
    pub str_measure17: Option<String>,
    #[serde(rename = "strMeasure18")]
    pub str_measure18: Option<String>,
    #[serde(rename = "strMeasure19")]
    pub str_measure19: Option<String>,
    #[serde(rename = "strMeasure20")]
    pub str_measure20: Option<String>,
    #[serde(rename = "strSource")]
    pub str_source: Option<String>,
    #[serde(rename = "strImageSource")]
    pub str_image_source: Option<String>,
    #[serde(rename = "strCreativeCommonsConfirmed")]
    pub str_creative_commons_confirmed: Option<String>,
    #[serde(rename = "dateModified")]
    pub date_modified: Option<String>,
}

impl Meal {
    pub fn get_ingredients(&self) -> Vec<(String, String)> {
        let ingredients = vec![
            (&self.str_ingredient1, &self.str_measure1),
            (&self.str_ingredient2, &self.str_measure2),
            (&self.str_ingredient3, &self.str_measure3),
            (&self.str_ingredient4, &self.str_measure4),
            (&self.str_ingredient5, &self.str_measure5),
            (&self.str_ingredient6, &self.str_measure6),
            (&self.str_ingredient7, &self.str_measure7),
            (&self.str_ingredient8, &self.str_measure8),
            (&self.str_ingredient9, &self.str_measure9),
            (&self.str_ingredient10, &self.str_measure10),
            (&self.str_ingredient11, &self.str_measure11),
            (&self.str_ingredient12, &self.str_measure12),
            (&self.str_ingredient13, &self.str_measure13),
            (&self.str_ingredient14, &self.str_measure14),
            (&self.str_ingredient15, &self.str_measure15),
            (&self.str_ingredient16, &self.str_measure16),
            (&self.str_ingredient17, &self.str_measure17),
            (&self.str_ingredient18, &self.str_measure18),
            (&self.str_ingredient19, &self.str_measure19),
            (&self.str_ingredient20, &self.str_measure20)
        ];

        ingredients
            .into_iter()
            .filter_map(|(ing, measure)| {
                match (ing, measure) {
                    (Some(i), Some(m)) if !i.trim().is_empty() && !m.trim().is_empty() => {
                        Some((i.clone(), m.clone()))
                    }
                    _ => None,
                }
            })
            .collect()
    }
}

#[derive(Clone)]
pub struct MealDbService {
    client: Arc<Client>,
    base_url: String,
}

impl MealDbService {
    pub fn new() -> Self {
        Self {
            client: Arc::new(Client::new()),
            base_url: "https://www.themealdb.com/api/json/v1/1".to_string(),
        }
    }

    pub async fn search_meals(&self, query: &str) -> Result<Vec<Meal>> {
        let url = format!("{}/search.php", self.base_url);

        let response = self.client
            .get(&url)
            .query(&[("s", query)])
            .send().await
            .context("Failed to send request to MealDB API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("MealDB API error: {} - {}", status, error_text);
        }

        let result = response
            .json::<MealsResponse>().await
            .context("Failed to parse MealDB API response")?;

        Ok(result.meals.unwrap_or_default())
    }

    pub async fn get_meal_by_id(&self, id: &str) -> Result<Option<Meal>> {
        let url = format!("{}/lookup.php", self.base_url);

        let response = self.client
            .get(&url)
            .query(&[("i", id)])
            .send().await
            .context("Failed to send request to MealDB API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("MealDB API error: {} - {}", status, error_text);
        }

        let result = response
            .json::<MealsResponse>().await
            .context("Failed to parse MealDB API response")?;

        Ok(result.meals.and_then(|mut meals| meals.pop()))
    }

    pub async fn get_random_meal(&self) -> Result<Option<Meal>> {
        let url = format!("{}/random.php", self.base_url);

        let response = self.client
            .get(&url)
            .send().await
            .context("Failed to send request to MealDB API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("MealDB API error: {} - {}", status, error_text);
        }

        let result = response
            .json::<MealsResponse>().await
            .context("Failed to parse MealDB API response")?;

        Ok(result.meals.and_then(|mut meals| meals.pop()))
    }

    pub async fn get_random_meals(&self, count: usize) -> Result<Vec<Meal>> {
        let mut meals = Vec::new();

        for _ in 0..count {
            if let Ok(Some(meal)) = self.get_random_meal().await {
                meals.push(meal);
            }
        }

        Ok(meals)
    }

    pub async fn filter_by_category(&self, category: &str) -> Result<Vec<Meal>> {
        let url = format!("{}/filter.php", self.base_url);

        let response = self.client
            .get(&url)
            .query(&[("c", category)])
            .send().await
            .context("Failed to send request to MealDB API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("MealDB API error: {} - {}", status, error_text);
        }

        let result = response
            .json::<MealsResponse>().await
            .context("Failed to parse MealDB API response")?;

        Ok(result.meals.unwrap_or_default())
    }

    pub async fn filter_by_area(&self, area: &str) -> Result<Vec<Meal>> {
        let url = format!("{}/filter.php", self.base_url);

        let response = self.client
            .get(&url)
            .query(&[("a", area)])
            .send().await
            .context("Failed to send request to MealDB API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("MealDB API error: {} - {}", status, error_text);
        }

        let result = response
            .json::<MealsResponse>().await
            .context("Failed to parse MealDB API response")?;

        Ok(result.meals.unwrap_or_default())
    }
}
