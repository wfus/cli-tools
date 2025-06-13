use crate::model_name::ModelName;
use crate::models::{ModelPricing, PricingMap};
use anyhow::Result;
use std::collections::HashMap;

// Hardcoded pricing as of June 2024
// Source: https://docs.anthropic.com/en/docs/about-claude/models
pub fn get_default_pricing() -> PricingMap {
    let mut pricing = HashMap::new();

    // Claude 3.5 Sonnet
    pricing.insert(
        ModelName::Claude35Sonnet,
        ModelPricing {
            input_per_million: 3.0,
            output_per_million: 15.0,
            cache_write_per_million: 3.75,
            cache_read_per_million: 0.30,
        },
    );

    // Claude 3.5 Haiku
    pricing.insert(
        ModelName::Claude35Haiku,
        ModelPricing {
            input_per_million: 0.80,
            output_per_million: 4.0,
            cache_write_per_million: 1.0,
            cache_read_per_million: 0.08,
        },
    );

    // Claude 3 Opus
    pricing.insert(
        ModelName::Claude3Opus,
        ModelPricing {
            input_per_million: 15.0,
            output_per_million: 75.0,
            cache_write_per_million: 18.75,
            cache_read_per_million: 1.50,
        },
    );

    // Claude Opus 4
    pricing.insert(
        ModelName::Claude4Opus,
        ModelPricing {
            input_per_million: 15.0,
            output_per_million: 75.0,
            cache_write_per_million: 18.75,
            cache_read_per_million: 1.50,
        },
    );

    // Claude Sonnet 4
    pricing.insert(
        ModelName::Claude4Sonnet,
        ModelPricing {
            input_per_million: 3.0,
            output_per_million: 15.0,
            cache_write_per_million: 3.75,
            cache_read_per_million: 0.30,
        },
    );

    // Claude 3.7 Sonnet (older version from logs)
    pricing.insert(
        ModelName::Claude37Sonnet,
        ModelPricing {
            input_per_million: 3.0,
            output_per_million: 15.0,
            cache_write_per_million: 3.75,
            cache_read_per_million: 0.30,
        },
    );

    // Claude 3 Haiku
    pricing.insert(
        ModelName::Claude3Haiku,
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

pub fn get_model_pricing<'a>(pricing_map: &'a PricingMap, model: &'a ModelName) -> Option<&'a ModelPricing> {
    // Try exact match first
    if let Some(pricing) = pricing_map.get(model) {
        return Some(pricing);
    }

    // For unknown models, try to match by family
    if let ModelName::Unknown(model_str) = model {
        let model_parsed = ModelName::from_model_string(model_str);
        if let Some(pricing) = pricing_map.get(&model_parsed) {
            return Some(pricing);
        }
        
        // If still unknown, try family matching
        let family = if model_str.contains("sonnet") {
            "sonnet"
        } else if model_str.contains("opus") {
            "opus"
        } else if model_str.contains("haiku") {
            "haiku"
        } else {
            return None;
        };
        
        // Get the first pricing for this family
        for (key, pricing) in pricing_map.iter() {
            if key.family() == family {
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
        assert!(pricing.contains_key(&ModelName::Claude35Sonnet));
        assert!(pricing.contains_key(&ModelName::Claude3Opus));
    }

    #[test]
    fn test_model_matching() {
        let pricing = get_default_pricing();
        
        // Test exact match
        assert!(get_model_pricing(&pricing, &ModelName::Claude35Sonnet).is_some());
        
        // Test unknown model parsing
        let unknown = ModelName::Unknown("claude-3-5-sonnet-unknown".to_string());
        assert!(get_model_pricing(&pricing, &unknown).is_some());
    }
}