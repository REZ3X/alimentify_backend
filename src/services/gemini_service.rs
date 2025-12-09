use anyhow::Result;
use base64::{ engine::general_purpose, Engine as _ };
use serde::{ Deserialize, Serialize };
use std::sync::Arc;

#[derive(Debug, Serialize)]
struct GeminiRequest {
    contents: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GenerationConfig>,
}

#[derive(Debug, Serialize)]
struct GenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    thinking_config: Option<ThinkingConfig>,
}

#[derive(Debug, Serialize)]
struct ThinkingConfig {
    thinking_level: String,
}

#[derive(Debug, Serialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum Part {
    Text {
        text: String,
    },
    InlineData {
        inline_data: InlineData,
    },
}

#[derive(Debug, Serialize)]
struct InlineData {
    mime_type: String,
    data: String,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Vec<Candidate>,
}

#[derive(Debug, Deserialize)]
struct Candidate {
    content: ResponseContent,
}

#[derive(Debug, Deserialize)]
struct ResponseContent {
    parts: Vec<ResponsePart>,
}

#[derive(Debug, Deserialize)]
struct ResponsePart {
    text: String,
}

#[derive(Clone)]
pub struct GeminiService {
    api_key: String,
    client: Arc<reqwest::Client>,
}

impl GeminiService {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: Arc::new(reqwest::Client::new()),
        }
    }

    pub async fn analyze_food_image(&self, image_data: &[u8], mime_type: &str) -> Result<String> {
        let base64_image = general_purpose::STANDARD.encode(image_data);

        let prompt =
            r#"Analyze this image for food content. Follow these steps:

STEP 1 - VALIDATION:
First, determine if the image contains actual human-edible food. 
- If the image shows non-food items (objects, animals, people, text, memes, inappropriate content, etc.), respond ONLY with this JSON:
{
  "is_valid_food": false,
  "error_type": "not_food",
  "message": "This image does not appear to contain food. Please upload a clear photo of a meal or food item."
}

- If the image shows something that is NOT typically consumed by humans (pet food, raw inedible items, toxic substances, etc.), respond ONLY with this JSON:
{
  "is_valid_food": false,
  "error_type": "not_edible",
  "message": "This item is not typically consumed as human food. Please upload a photo of an edible meal or food item."
}

- If the image is inappropriate, offensive, or contains sensitive content, respond ONLY with this JSON:
{
  "is_valid_food": false,
  "error_type": "inappropriate",
  "message": "This image cannot be processed. Please upload an appropriate photo of food."
}

STEP 2 - ANALYSIS (only if validation passes):
If the image contains valid, human-edible food, provide detailed nutritional information in this JSON format:

{
  "is_valid_food": true,
  "food_name": "name of the food item",
  "serving_size": "typical serving size",
  "calories": "estimated calories per serving",
  "macronutrients": {
    "protein": "grams of protein",
    "carbohydrates": "grams of carbohydrates",
    "fat": "grams of fat",
    "fiber": "grams of fiber"
  },
  "micronutrients": {
    "vitamins": ["list of significant vitamins"],
    "minerals": ["list of significant minerals"]
  },
  "health_score": "score from 1-10 based on nutritional value",
  "health_notes": "brief notes about health benefits or concerns",
  "dietary_info": {
    "is_vegetarian": true/false,
    "is_vegan": true/false,
    "is_gluten_free": true/false,
    "allergens": ["list of common allergens present"]
  },
  "recommendations": "suggestions for healthier alternatives or complementary foods"
}

Be accurate based on visual analysis. If you cannot clearly identify the food, indicate uncertainty in your response but still provide estimates if it appears to be food."#;

        let request_body = GeminiRequest {
            contents: vec![Content {
                parts: vec![
                    Part::Text {
                        text: prompt.to_string(),
                    },
                    Part::InlineData {
                        inline_data: InlineData {
                            mime_type: mime_type.to_string(),
                            data: base64_image,
                        },
                    }
                ],
            }],
            generation_config: Some(GenerationConfig {
                thinking_config: Some(ThinkingConfig {
                    thinking_level: "low".to_string(),
                }),
            }),
        };

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-3-pro-preview:generateContent?key={}",
            self.api_key
        );

        tracing::info!("Sending request to Gemini 3 Pro Preview API for food analysis");

        let response = self.client.post(&url).json(&request_body).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            tracing::error!("Gemini API error: {} - {}", status, error_text);
            anyhow::bail!("Gemini API request failed: {} - {}", status, error_text);
        }

        let gemini_response: GeminiResponse = response.json().await?;

        let analysis_text = gemini_response.candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .map(|p| p.text.clone())
            .ok_or_else(|| anyhow::anyhow!("No response from Gemini API"))?;

        tracing::info!("Successfully received analysis from Gemini API");

        Ok(analysis_text)
    }

    pub async fn quick_food_check(&self, image_data: &[u8], mime_type: &str) -> Result<String> {
        let base64_image = general_purpose::STANDARD.encode(image_data);

        let prompt =
            "Identify this food and provide a brief health assessment (1-2 sentences) including estimated calories and whether it's generally healthy or not.";

        let request_body = GeminiRequest {
            contents: vec![Content {
                parts: vec![
                    Part::Text {
                        text: prompt.to_string(),
                    },
                    Part::InlineData {
                        inline_data: InlineData {
                            mime_type: mime_type.to_string(),
                            data: base64_image,
                        },
                    }
                ],
            }],
            generation_config: Some(GenerationConfig {
                thinking_config: Some(ThinkingConfig {
                    thinking_level: "low".to_string(),
                }),
            }),
        };

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-3-pro-preview:generateContent?key={}",
            self.api_key
        );

        let response = self.client.post(&url).json(&request_body).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            anyhow::bail!("Gemini API request failed: {} - {}", status, error_text);
        }

        let gemini_response: GeminiResponse = response.json().await?;

        let analysis_text = gemini_response.candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .map(|p| p.text.clone())
            .ok_or_else(|| anyhow::anyhow!("No response from Gemini API"))?;

        Ok(analysis_text)
    }

    pub async fn get_text_response(&self, prompt: &str) -> Result<String> {
        let request_body = GeminiRequest {
            contents: vec![Content {
                parts: vec![Part::Text {
                    text: prompt.to_string(),
                }],
            }],
            generation_config: Some(GenerationConfig {
                thinking_config: Some(ThinkingConfig {
                    thinking_level: "low".to_string(),
                }),
            }),
        };

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-3-pro-preview:generateContent?key={}",
            self.api_key
        );

        let response = self.client.post(&url).json(&request_body).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            anyhow::bail!("Gemini API request failed: {} - {}", status, error_text);
        }

        let gemini_response: GeminiResponse = response.json().await?;

        let text = gemini_response.candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .map(|p| p.text.clone())
            .ok_or_else(|| anyhow::anyhow!("No response from Gemini API"))?;

        Ok(text)
    }

    pub async fn analyze_food_from_text(
        &self,
        food_description: &str
    ) -> Result<serde_json::Value> {
        let inappropriate_keywords = [
            "human", "person", "people", "body", "flesh", "blood", "organ",
            "cannibal", "corpse", "dead", "kill", "murder", "poison",
            "toxic", "dangerous", "harmful", "drug", "narcotic",
            "feces", "urine", "waste", "dirt", "sand", "rock", "metal",
            "plastic", "glass", "wood", "paper", "rubber", "chemical"
        ];
        
        let description_lower = food_description.to_lowercase();

        for keyword in &inappropriate_keywords {
            if description_lower.contains(keyword) {
                return Ok(serde_json::json!({
                    "is_valid_food": false,
                    "error_type": "inappropriate",
                    "message": "This doesn't appear to be a valid food item. Please enter an actual food or meal."
                }));
            }
        }
        
        let prompt =
            format!(r#"Analyze the following food description and provide detailed nutrition information.

Food Description: {}

IMPORTANT: First, determine if this is a valid, human-edible food item.

If the description is NOT a valid food (e.g., non-food objects, inappropriate content, inedible items, or anything that shouldn't be consumed), respond ONLY with this JSON:
{{
    "is_valid_food": false,
    "error_type": "not_food",
    "message": "This doesn't appear to be a valid food item. Please enter an actual food or meal."
}}

If it IS a valid food, provide the response as a valid JSON object with this exact structure:
{{
    "is_valid_food": true,
    "food_name": "the food name",
    "calories": <number>,
    "protein_g": <number>,
    "carbs_g": <number>,
    "fat_g": <number>,
    "serving_size": "serving description"
}}

Guidelines:
1. Use reasonable estimates for nutrition values based on standard servings
2. If a portion size is mentioned (e.g., "200g", "2 slices"), use that for calculations
3. If no portion is specified, assume a standard serving size
4. All numeric values should be numbers (not strings)
5. serving_size should describe what the nutrition values represent
6. Be accurate but reasonable with estimates

Return ONLY the JSON object, nothing else."#, food_description);

        let response_text = self.get_text_response(&prompt).await?;
        
        let response_lower = response_text.to_lowercase();
        let safety_indicators = [
            "cannot fulfill", "i cannot", "i'm not able", "i am not able",
            "safety guidelines", "prohibited", "harmful", "violence",
            "self-harm", "cannibalism", "inappropriate", "i'm sorry",
            "i apologize", "not appropriate", "refuse to"
        ];
        
        for indicator in &safety_indicators {
            if response_lower.contains(indicator) {
                tracing::info!("Detected safety response from Gemini, returning user-friendly message");
                return Ok(serde_json::json!({
                    "is_valid_food": false,
                    "error_type": "inappropriate",
                    "message": "This doesn't appear to be a valid food item. Please enter an actual food or meal."
                }));
            }
        }

        let json_str = if let Some(start) = response_text.find('{') {
            if let Some(end) = response_text.rfind('}') {
                &response_text[start..=end]
            } else {
                &response_text
            }
        } else {
            tracing::info!("No JSON found in response, treating as invalid food");
            return Ok(serde_json::json!({
                "is_valid_food": false,
                "error_type": "parse_error",
                "message": "Could not analyze this item. Please try a different food description."
            }));
        };

        let nutrition_data: serde_json::Value = serde_json
            ::from_str(json_str)
            .map_err(|e| {
                tracing::warn!("Failed to parse JSON: {}. Response was: {}", e, response_text);
                anyhow::anyhow!(
                    "Failed to parse AI response as JSON: {}. Response was: {}",
                    e,
                    response_text
                )
            })?;

        Ok(nutrition_data)
    }
}
