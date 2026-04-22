//! Fallback strategies when agent is uncertain

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FallbackStrategy {
    AskClarification,
    ProvidePartialAnswer,
    SuggestSearch,
    OfferHandoff,
    ApologizeAndRetry,
    DefaultResponse,
}

impl FallbackStrategy {
    pub fn template(&self) -> &str {
        match self {
            FallbackStrategy::AskClarification => {
                "I'm not entirely sure I understood your question correctly. Could you clarify: {specific_question}?"
            },
            FallbackStrategy::ProvidePartialAnswer => {
                "Based on what I know, {partial_info}. However, I should mention that {uncertainty}."
            },
            FallbackStrategy::SuggestSearch => {
                "I don't have enough information to answer that accurately. Would you like me to search for more details on {topic}?"
            },
            FallbackStrategy::OfferHandoff => {
                "This seems like a complex question that might be better handled by a human specialist. Would you like me to connect you with someone?"
            },
            FallbackStrategy::ApologizeAndRetry => {
                "I apologize, but I'm having trouble providing a accurate answer. Let me try again with a different approach."
            },
            FallbackStrategy::DefaultResponse => {
                "I understand you're asking about {topic}, but I don't have enough confidence in my answer to provide it. Could we discuss something else?"
            },
        }
    }

    pub fn confidence_threshold(&self) -> f32 {
        match self {
            FallbackStrategy::AskClarification => 0.5,
            FallbackStrategy::ProvidePartialAnswer => 0.4,
            FallbackStrategy::SuggestSearch => 0.3,
            FallbackStrategy::OfferHandoff => 0.2,
            FallbackStrategy::ApologizeAndRetry => 0.15,
            FallbackStrategy::DefaultResponse => 0.1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FallbackTrigger {
    LowConfidence,
    OutOfContext,
    UnknownTopic,
    AmbiguousRequest,
    SensitiveContent,
    UserDissatisfaction,
    RepeatedFailure,
}

impl FallbackTrigger {
    pub fn should_trigger(&self, confidence: f32) -> bool {
        match self {
            FallbackTrigger::LowConfidence => confidence < 0.6,
            FallbackTrigger::OutOfContext => confidence < 0.5,
            FallbackTrigger::UnknownTopic => confidence < 0.4,
            FallbackTrigger::AmbiguousRequest => confidence < 0.5,
            FallbackTrigger::SensitiveContent => confidence < 0.8,
            FallbackTrigger::UserDissatisfaction => true,
            FallbackTrigger::RepeatedFailure => true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackContext {
    pub session_id: String,
    pub original_query: String,
    pub detected_topic: Option<String>,
    pub confidence_scores: ConfidenceScores,
    pub previous_attempts: usize,
    pub user_sentiment: Option<f32>,
    pub topic_category: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceScores {
    pub overall: f32,
    pub semantic: f32,
    pub factual: f32,
    pub coherence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackResult {
    pub strategy: FallbackStrategy,
    pub response: String,
    pub confidence: f32,
    pub handoff_triggered: bool,
    pub retry_suggested: bool,
    pub alternative_actions: Vec<AlternativeAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlternativeAction {
    pub action_type: AlternativeActionType,
    pub label: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlternativeActionType {
    Search,
    Handoff,
    Retry,
    Simplify,
    ProvideExamples,
}

pub struct FallbackEngine {
    strategies: Vec<FallbackStrategy>,
    #[allow(dead_code)]
    trigger_thresholds: std::collections::HashMap<FallbackTrigger, f32>,
}

impl Default for FallbackEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl FallbackEngine {
    pub fn new() -> Self {
        Self {
            strategies: vec![
                FallbackStrategy::AskClarification,
                FallbackStrategy::ProvidePartialAnswer,
                FallbackStrategy::SuggestSearch,
                FallbackStrategy::OfferHandoff,
                FallbackStrategy::ApologizeAndRetry,
                FallbackStrategy::DefaultResponse,
            ],
            trigger_thresholds: vec![
                (FallbackTrigger::LowConfidence, 0.6),
                (FallbackTrigger::OutOfContext, 0.5),
                (FallbackTrigger::UnknownTopic, 0.4),
                (FallbackTrigger::SensitiveContent, 0.8),
            ].into_iter().collect(),
        }
    }

    pub fn select_strategy(&self, context: &FallbackContext) -> FallbackStrategy {
        let confidence = context.confidence_scores.overall;

        if context.previous_attempts >= 3 {
            return FallbackStrategy::OfferHandoff;
        }

        if let Some(sentiment) = context.user_sentiment {
            if sentiment < 0.3 {
                return FallbackStrategy::OfferHandoff;
            }
        }

        for strategy in &self.strategies {
            if confidence >= strategy.confidence_threshold() {
                return strategy.clone();
            }
        }

        FallbackStrategy::DefaultResponse
    }

    pub fn generate_response(
        &self,
        strategy: &FallbackStrategy,
        context: &FallbackContext,
    ) -> String {
        let template = strategy.template();
        
        let mut response = template
            .replace("{topic}", &context.detected_topic.clone().unwrap_or_else(|| "this".to_string()))
            .replace("{specific_question}", &self.generate_clarification_question(context))
            .replace("{partial_info}", &self.generate_partial_info(context))
            .replace("{uncertainty}", &self.generate_uncertainty_statement(context));

        if matches!(strategy, FallbackStrategy::ApologizeAndRetry) {
            response = format!("I apologize, but I'm having trouble providing an accurate answer. {}", response);
        }

        response
    }

    fn generate_clarification_question(&self, context: &FallbackContext) -> String {
        if context.previous_attempts > 0 {
            "Could you provide more specific details about what you're looking for?".to_string()
        } else {
            match context.topic_category.as_deref() {
                Some("technical") => "Could you share more specific technical details or error messages?",
                Some("business") => "Could you provide more context about your business needs?",
                Some("personal") => "Could you share more about your specific situation?",
                _ => "Could you tell me more about what you're looking for?",
            }.to_string()
        }
    }

    fn generate_partial_info(&self, context: &FallbackContext) -> String {
        let topic = context.detected_topic.clone().unwrap_or_default();
        format!("here's what I understand about {}", topic)
    }

    fn generate_uncertainty_statement(&self, context: &FallbackContext) -> String {
        if context.previous_attempts > 0 {
            "I want to make sure I give you accurate information.".to_string()
        } else {
            "my understanding might not be complete or up-to-date.".to_string()
        }
    }

    pub fn execute(&self, context: &FallbackContext) -> FallbackResult {
        let strategy = self.select_strategy(context);
        let response = self.generate_response(&strategy, context);
        
        let handoff_triggered = matches!(
            strategy,
            FallbackStrategy::OfferHandoff
        );

        let retry_suggested = matches!(
            strategy,
            FallbackStrategy::ApologizeAndRetry | FallbackStrategy::AskClarification
        );

        let alternative_actions = self.generate_alternative_actions(&strategy, context);

        FallbackResult {
            strategy,
            response,
            confidence: context.confidence_scores.overall,
            handoff_triggered,
            retry_suggested,
            alternative_actions,
        }
    }

    fn generate_alternative_actions(
        &self,
        strategy: &FallbackStrategy,
        context: &FallbackContext,
    ) -> Vec<AlternativeAction> {
        let mut actions = Vec::new();

        if !matches!(strategy, FallbackStrategy::SuggestSearch) {
            actions.push(AlternativeAction {
                action_type: AlternativeActionType::Search,
                label: "Search for more info".to_string(),
                description: "Let me search for additional information".to_string(),
            });
        }

        if !matches!(strategy, FallbackStrategy::OfferHandoff) {
            actions.push(AlternativeAction {
                action_type: AlternativeActionType::Handoff,
                label: "Talk to a human".to_string(),
                description: "Connect with a human agent for personalized help".to_string(),
            });
        }

        if context.previous_attempts == 0 {
            actions.push(AlternativeAction {
                action_type: AlternativeActionType::Retry,
                label: "Try again".to_string(),
                description: "Let me attempt to answer differently".to_string(),
            });
        }

        actions.push(AlternativeAction {
            action_type: AlternativeActionType::Simplify,
            label: "Simplify my question".to_string(),
            description: "Ask a simpler or different question".to_string(),
        });

        actions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strategy_selection() {
        let engine = FallbackEngine::new();

        let low_confidence = FallbackContext {
            session_id: "test".to_string(),
            original_query: "test".to_string(),
            detected_topic: Some("AI".to_string()),
            confidence_scores: ConfidenceScores {
                overall: 0.3,
                semantic: 0.4,
                factual: 0.2,
                coherence: 0.3,
            },
            previous_attempts: 0,
            user_sentiment: None,
            topic_category: Some("technical".to_string()),
        };

        let strategy = engine.select_strategy(&low_confidence);
        assert!(matches!(strategy, FallbackStrategy::SuggestSearch | FallbackStrategy::ProvidePartialAnswer));
    }

    #[test]
    fn test_handoff_on_repeated_failure() {
        let engine = FallbackEngine::new();

        let context = FallbackContext {
            session_id: "test".to_string(),
            original_query: "test".to_string(),
            detected_topic: None,
            confidence_scores: ConfidenceScores {
                overall: 0.7,
                semantic: 0.8,
                factual: 0.7,
                coherence: 0.6,
            },
            previous_attempts: 3,
            user_sentiment: None,
            topic_category: None,
        };

        let strategy = engine.select_strategy(&context);
        assert!(matches!(strategy, FallbackStrategy::OfferHandoff));
    }

    #[test]
    fn test_fallback_execution() {
        let engine = FallbackEngine::new();

        let context = FallbackContext {
            session_id: "test".to_string(),
            original_query: "How do neural networks work?".to_string(),
            detected_topic: Some("neural networks".to_string()),
            confidence_scores: ConfidenceScores {
                overall: 0.5,
                semantic: 0.6,
                factual: 0.5,
                coherence: 0.4,
            },
            previous_attempts: 0,
            user_sentiment: None,
            topic_category: Some("technical".to_string()),
        };

        let result = engine.execute(&context);
        
        assert!(!result.response.is_empty());
        assert!(!result.alternative_actions.is_empty());
    }
}
