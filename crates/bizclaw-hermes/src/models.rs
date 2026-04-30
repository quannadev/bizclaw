//! # Hermes Models
//! 
//! Supported Hermes models configuration

use serde::{Deserialize, Serialize};

/// Model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub name: String,
    pub quantization: Option<String>,
    pub context_length: u32,
    pub recommended_for: Vec<String>,
}

impl ModelConfig {
    pub fn hermes_2_pro_llama_3_8b() -> Self {
        Self {
            name: " NousResearch/Hermes-2-Pro-Llama-3-8B".to_string(),
            quantization: Some("Q4_K_M".to_string()),
            context_length: 8192,
            recommended_for: vec![
                "tool_calling".to_string(),
                "vietnamese".to_string(),
                "instruction_following".to_string(),
            ],
        }
    }

    pub fn hermes_3_llama_3_1_8b() -> Self {
        Self {
            name: " NousResearch/Hermes-3-Llama-3.1-8B".to_string(),
            quantization: Some("Q4_K_M".to_string()),
            context_length: 128000,
            recommended_for: vec![
                "long_context".to_string(),
                "function_calling".to_string(),
                "vietnamese".to_string(),
            ],
        }
    }

    pub fn hermes_3_theta_8b() -> Self {
        Self {
            name: "NousResearch/Hermes-3-Theta-8B".to_string(),
            quantization: Some("Q4_K_M".to_string()),
            context_length: 128000,
            recommended_for: vec![
                "high_quality".to_string(),
                "reasoning".to_string(),
                "coding".to_string(),
            ],
        }
    }
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self::hermes_2_pro_llama_3_8b()
    }
}

/// Hermes model wrapper
pub struct HermesModel {
    config: ModelConfig,
}

impl HermesModel {
    pub fn new(config: ModelConfig) -> Self {
        Self { config }
    }

    pub fn from_preset(preset: &str) -> Option<Self> {
        match preset {
            "hermes-2-pro" => Some(Self::new(ModelConfig::hermes_2_pro_llama_3_8b())),
            "hermes-3" => Some(Self::new(ModelConfig::hermes_3_llama_3_1_8b())),
            "hermes-3-theta" => Some(Self::new(ModelConfig::hermes_3_theta_8b())),
            _ => None,
        }
    }

    pub fn config(&self) -> &ModelConfig {
        &self.config
    }
}
