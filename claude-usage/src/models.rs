use crate::model_name::ModelName;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    #[serde(rename = "type")]
    pub entry_type: String,
    pub uuid: String,
    #[serde(rename = "parentUuid")]
    pub parent_uuid: Option<String>,
    pub timestamp: DateTime<Utc>,
    #[serde(rename = "sessionId")]
    pub session_id: String,
    #[serde(rename = "requestId")]
    pub request_id: Option<String>,
    pub version: Option<String>,
    pub message: Option<Message>,
    #[serde(rename = "isSidechain")]
    pub is_sidechain: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub role: String,
    #[serde(with = "model_name_serde")]
    pub model: ModelName,
    pub usage: Option<TokenUsage>,
}

// Custom serde implementation to handle model as string in JSON
mod model_name_serde {
    use super::*;
    use serde::{Deserializer, Serializer};
    
    pub fn serialize<S>(model: &ModelName, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&model.canonical_string())
    }
    
    pub fn deserialize<'de, D>(deserializer: D) -> Result<ModelName, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(ModelName::from_model_string(&s))
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    #[serde(default)]
    pub cache_creation_input_tokens: u64,
    #[serde(default)]
    pub cache_read_input_tokens: u64,
    pub service_tier: Option<String>,
}

impl TokenUsage {
    pub fn total_tokens(&self) -> u64 {
        self.input_tokens
            + self.output_tokens
            + self.cache_creation_input_tokens
            + self.cache_read_input_tokens
    }

    pub fn add(&mut self, other: &TokenUsage) {
        self.input_tokens += other.input_tokens;
        self.output_tokens += other.output_tokens;
        self.cache_creation_input_tokens += other.cache_creation_input_tokens;
        self.cache_read_input_tokens += other.cache_read_input_tokens;
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct UsageStats {
    pub model: ModelName,
    pub date: DateTime<Utc>,
    pub usage: TokenUsage,
    pub request_count: u64,
    pub cost_usd: f64,
}

#[derive(Debug, Clone)]
pub struct ModelPricing {
    pub input_per_million: f64,
    pub output_per_million: f64,
    pub cache_write_per_million: f64,
    pub cache_read_per_million: f64,
}

impl ModelPricing {
    pub fn calculate_cost(&self, usage: &TokenUsage) -> f64 {
        (usage.input_tokens as f64 * self.input_per_million
            + usage.output_tokens as f64 * self.output_per_million
            + usage.cache_creation_input_tokens as f64 * self.cache_write_per_million
            + usage.cache_read_input_tokens as f64 * self.cache_read_per_million)
            / 1_000_000.0
    }
}

pub type PricingMap = HashMap<ModelName, ModelPricing>;