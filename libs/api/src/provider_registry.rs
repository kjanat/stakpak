use crate::LLMProvider;
use stakpak_shared::models::integrations::openai::AgentModel;

/// Information about a specific model offered by a provider
#[derive(Debug, Clone)]
pub struct ModelInfo {
    /// The enum variant used internally (Smart, Eco, etc.)
    pub agent_model: AgentModel,
    /// Display name shown to users in the TUI
    pub display_name: String,
    /// The actual model ID used by the provider's API
    pub api_model_id: String,
    /// Short description of the model
    pub description: String,
}

/// Information about an LLM provider
#[derive(Debug, Clone)]
pub struct ProviderInfo {
    /// Provider enum variant
    pub provider: LLMProvider,
    /// Display name
    pub display_name: String,
    /// Available models for this provider
    pub models: Vec<ModelInfo>,
}

impl ProviderInfo {
    /// Get model info by AgentModel enum
    pub fn get_model(&self, agent_model: &AgentModel) -> Option<&ModelInfo> {
        self.models
            .iter()
            .find(|m| &m.agent_model == agent_model)
    }

    /// Get the API model ID for an AgentModel
    pub fn get_api_model_id(&self, agent_model: &AgentModel) -> Option<String> {
        self.get_model(agent_model)
            .map(|m| m.api_model_id.clone())
    }

    /// Get all available model display names
    pub fn get_model_names(&self) -> Vec<String> {
        self.models.iter().map(|m| m.display_name.clone()).collect()
    }
}

/// Get provider information for a specific provider
pub fn get_provider_info(provider: &LLMProvider) -> ProviderInfo {
    match provider {
        LLMProvider::Stakpak => ProviderInfo {
            provider: LLMProvider::Stakpak,
            display_name: "Stakpak".to_string(),
            models: vec![
                ModelInfo {
                    agent_model: AgentModel::Smart,
                    display_name: "smart".to_string(),
                    api_model_id: "smart".to_string(),
                    description: "Claude Sonnet 4 - Balanced performance".to_string(),
                },
                ModelInfo {
                    agent_model: AgentModel::Eco,
                    display_name: "eco".to_string(),
                    api_model_id: "eco".to_string(),
                    description: "Claude Haiku 4 - Fast and efficient".to_string(),
                },
            ],
        },
        LLMProvider::Anthropic => ProviderInfo {
            provider: LLMProvider::Anthropic,
            display_name: "Anthropic".to_string(),
            models: vec![
                ModelInfo {
                    agent_model: AgentModel::Opus,
                    display_name: "Claude Opus 4".to_string(),
                    api_model_id: "claude-opus-4-20250514".to_string(),
                    description: "Most capable model for complex tasks".to_string(),
                },
                ModelInfo {
                    agent_model: AgentModel::Smart,
                    display_name: "Claude Sonnet 4".to_string(),
                    api_model_id: "claude-sonnet-4-20250514".to_string(),
                    description: "Balanced performance and speed".to_string(),
                },
                ModelInfo {
                    agent_model: AgentModel::Eco,
                    display_name: "Claude Haiku 4".to_string(),
                    api_model_id: "claude-haiku-4-20250605".to_string(),
                    description: "Fast and efficient for simple tasks".to_string(),
                },
            ],
        },
    }
}

/// Get all available providers
pub fn get_all_providers() -> Vec<ProviderInfo> {
    vec![
        get_provider_info(&LLMProvider::Stakpak),
        get_provider_info(&LLMProvider::Anthropic),
    ]
}
