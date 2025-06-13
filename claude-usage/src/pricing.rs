use crate::models::{ModelPricing, PricingMap};
use anyhow::Result;
use std::collections::HashMap;

// Hardcoded pricing as of June 2024
// Source: https://docs.anthropic.com/en/docs/about-claude/models
pub fn get_default_pricing() -> PricingMap {
    let mut pricing = HashMap::new();

    // Claude 3.5 Sonnet
    pricing.insert(
        "claude-3-5-sonnet-20241022".to_string(),
        ModelPricing {
            input_per_million: 3.0,
            output_per_million: 15.0,
            cache_write_per_million: 3.75,
            cache_read_per_million: 0.30,
        },
    );

    pricing.insert(
        "claude-3-5-sonnet-20240620".to_string(),
        ModelPricing {
            input_per_million: 3.0,
            output_per_million: 15.0,
            cache_write_per_million: 3.75,
            cache_read_per_million: 0.30,
        },
    );

    // Claude 3.5 Haiku
    pricing.insert(
        "claude-3-5-haiku-20241022".to_string(),
        ModelPricing {
            input_per_million: 0.80,
            output_per_million: 4.0,
            cache_write_per_million: 1.0,
            cache_read_per_million: 0.08,
        },
    );

    // Claude 3 Opus
    pricing.insert(
        "claude-3-opus-20240229".to_string(),
        ModelPricing {
            input_per_million: 15.0,
            output_per_million: 75.0,
            cache_write_per_million: 18.75,
            cache_read_per_million: 1.50,
        },
    );

    // Claude Opus 4
    pricing.insert(
        "claude-opus-4-20250514".to_string(),
        ModelPricing {
            input_per_million: 15.0,
            output_per_million: 75.0,
            cache_write_per_million: 18.75,
            cache_read_per_million: 1.50,
        },
    );

    // Claude Sonnet 4
    pricing.insert(
        "claude-sonnet-4-20250514".to_string(),
        ModelPricing {
            input_per_million: 3.0,
            output_per_million: 15.0,
            cache_write_per_million: 3.75,
            cache_read_per_million: 0.30,
        },
    );

    // Claude 3.7 Sonnet (older version from logs)
    pricing.insert(
        "claude-3-7-sonnet-20250219".to_string(),
        ModelPricing {
            input_per_million: 3.0,
            output_per_million: 15.0,
            cache_write_per_million: 3.75,
            cache_read_per_million: 0.30,
        },
    );

    // Claude 3 Haiku
    pricing.insert(
        "claude-3-haiku-20240307".to_string(),
        ModelPricing {
            input_per_million: 0.25,
            output_per_million: 1.25,
            cache_write_per_million: 0.30,
            cache_read_per_million: 0.03,
        },
    );

    pricing
}

pub async fn fetch_latest_pricing() -> Result<PricingMap> {
    // In a real implementation, this would fetch from Anthropic's API
    // For now, we'll just return the hardcoded pricing
    // This is a placeholder for future API integration
    
    println!("Note: Using hardcoded pricing. API integration coming soon.");
    Ok(get_default_pricing())
}

pub fn get_model_pricing<'a>(pricing_map: &'a PricingMap, model: &'a str) -> Option<&'a ModelPricing> {
    // Try exact match first
    if let Some(pricing) = pricing_map.get(model) {
        return Some(pricing);
    }

    // Try to match by model family
    if model.contains("sonnet") {
        // Get the latest sonnet pricing
        for (key, pricing) in pricing_map.iter() {
            if key.contains("sonnet") && key.contains("20241022") {
                return Some(pricing);
            }
        }
    } else if model.contains("opus") {
        // Get the latest opus pricing
        for (key, pricing) in pricing_map.iter() {
            if key.contains("opus") {
                return Some(pricing);
            }
        }
    } else if model.contains("haiku") {
        // Get the latest haiku pricing
        for (key, pricing) in pricing_map.iter() {
            if key.contains("haiku") && key.contains("20241022") {
                return Some(pricing);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_pricing() {
        let pricing = get_default_pricing();
        assert!(pricing.contains_key("claude-3-5-sonnet-20241022"));
        assert!(pricing.contains_key("claude-3-opus-20240229"));
    }

    #[test]
    fn test_model_matching() {
        let pricing = get_default_pricing();
        
        // Test exact match
        assert!(get_model_pricing(&pricing, "claude-3-5-sonnet-20241022").is_some());
        
        // Test family matching
        assert!(get_model_pricing(&pricing, "claude-3-5-sonnet-unknown").is_some());
    }
}