use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum ModelName {
    // Claude 3 models
    Claude3Opus,
    Claude3Sonnet,
    Claude3Haiku,
    
    // Claude 3.5 models
    Claude35Sonnet,
    Claude35Haiku,
    
    // Claude 3.7 models (older)
    Claude37Sonnet,
    
    // Claude 4 models
    Claude4Opus,
    Claude4Sonnet,
    
    // Special
    Synthetic,
    
    // Forward compatibility
    Unknown(String),
}

impl ModelName {
    /// Parse a model string into a ModelName
    pub fn from_model_string(s: &str) -> Self {
        match s {
            // Claude 3 models
            "claude-3-opus-20240229" => ModelName::Claude3Opus,
            "claude-3-sonnet-20240229" => ModelName::Claude3Sonnet,
            "claude-3-haiku-20240307" => ModelName::Claude3Haiku,
            
            // Claude 3.5 models
            s if s.starts_with("claude-3-5-sonnet-") => ModelName::Claude35Sonnet,
            s if s.starts_with("claude-3-5-haiku-") => ModelName::Claude35Haiku,
            
            // Claude 3.7 models
            s if s.starts_with("claude-3-7-sonnet-") => ModelName::Claude37Sonnet,
            
            // Claude 4 models
            "claude-opus-4-20250514" => ModelName::Claude4Opus,
            "claude-sonnet-4-20250514" => ModelName::Claude4Sonnet,
            
            // Special
            "<synthetic>" => ModelName::Synthetic,
            
            // Unknown
            _ => ModelName::Unknown(s.to_string()),
        }
    }
    
    /// Get a canonical string representation for pricing lookups
    pub fn canonical_string(&self) -> String {
        match self {
            ModelName::Claude3Opus => "claude-3-opus-20240229".to_string(),
            ModelName::Claude3Sonnet => "claude-3-sonnet-20240229".to_string(),
            ModelName::Claude3Haiku => "claude-3-haiku-20240307".to_string(),
            ModelName::Claude35Sonnet => "claude-3-5-sonnet-20241022".to_string(),
            ModelName::Claude35Haiku => "claude-3-5-haiku-20241022".to_string(),
            ModelName::Claude37Sonnet => "claude-3-7-sonnet-20250219".to_string(),
            ModelName::Claude4Opus => "claude-opus-4-20250514".to_string(),
            ModelName::Claude4Sonnet => "claude-sonnet-4-20250514".to_string(),
            ModelName::Synthetic => "<synthetic>".to_string(),
            ModelName::Unknown(s) => s.clone(),
        }
    }
    
    /// Get the model family (opus, sonnet, haiku)
    pub fn family(&self) -> &str {
        match self {
            ModelName::Claude3Opus | ModelName::Claude4Opus => "opus",
            ModelName::Claude3Sonnet | ModelName::Claude35Sonnet | ModelName::Claude37Sonnet | ModelName::Claude4Sonnet => "sonnet",
            ModelName::Claude3Haiku | ModelName::Claude35Haiku => "haiku",
            ModelName::Synthetic => "synthetic",
            ModelName::Unknown(_) => "unknown",
        }
    }
    
    /// Check if this is a synthetic model
    pub fn is_synthetic(&self) -> bool {
        matches!(self, ModelName::Synthetic)
    }
}

impl fmt::Display for ModelName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModelName::Claude3Opus => write!(f, "Claude 3 Opus"),
            ModelName::Claude3Sonnet => write!(f, "Claude 3 Sonnet"),
            ModelName::Claude3Haiku => write!(f, "Claude 3 Haiku"),
            ModelName::Claude35Sonnet => write!(f, "Claude 3.5 Sonnet"),
            ModelName::Claude35Haiku => write!(f, "Claude 3.5 Haiku"),
            ModelName::Claude37Sonnet => write!(f, "Claude 3.7 Sonnet"),
            ModelName::Claude4Opus => write!(f, "Claude 4 Opus"),
            ModelName::Claude4Sonnet => write!(f, "Claude 4 Sonnet"),
            ModelName::Synthetic => write!(f, "Synthetic"),
            ModelName::Unknown(s) => write!(f, "{}", s),
        }
    }
}

impl FromStr for ModelName {
    type Err = std::convert::Infallible;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(ModelName::from_model_string(s))
    }
}

impl TryFrom<String> for ModelName {
    type Error = std::convert::Infallible;
    
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Ok(ModelName::from_model_string(&s))
    }
}

impl From<ModelName> for String {
    fn from(model: ModelName) -> String {
        model.canonical_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_model_parsing() {
        assert_eq!(ModelName::from_model_string("claude-opus-4-20250514"), ModelName::Claude4Opus);
        assert_eq!(ModelName::from_model_string("claude-3-5-sonnet-20241022"), ModelName::Claude35Sonnet);
        assert_eq!(ModelName::from_model_string("unknown-model"), ModelName::Unknown("unknown-model".to_string()));
    }
    
    #[test]
    fn test_model_family() {
        assert_eq!(ModelName::Claude4Opus.family(), "opus");
        assert_eq!(ModelName::Claude35Sonnet.family(), "sonnet");
        assert_eq!(ModelName::Claude3Haiku.family(), "haiku");
    }
    
    #[test]
    fn test_serde_roundtrip() {
        let model = ModelName::Claude4Opus;
        let json = serde_json::to_string(&model).unwrap();
        let parsed: ModelName = serde_json::from_str(&json).unwrap();
        assert_eq!(model, parsed);
    }
}