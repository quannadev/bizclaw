//! Conversation flow designer

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlowState {
    Idle,
    Listening,
    Processing,
    Responding,
    AwaitingConfirmation,
    HandoffPending,
    HandoffActive,
    Error,
    Completed,
}

impl FlowState {
    pub fn can_transition_to(&self, target: &FlowState) -> bool {
        match (self, target) {
            (FlowState::Idle, FlowState::Listening) => true,
            (FlowState::Listening, FlowState::Processing) => true,
            (FlowState::Listening, FlowState::Error) => true,
            (FlowState::Processing, FlowState::Responding) => true,
            (FlowState::Processing, FlowState::AwaitingConfirmation) => true,
            (FlowState::Processing, FlowState::HandoffPending) => true,
            (FlowState::Processing, FlowState::Error) => true,
            (FlowState::Responding, FlowState::Listening) => true,
            (FlowState::Responding, FlowState::Completed) => true,
            (FlowState::Responding, FlowState::AwaitingConfirmation) => true,
            (FlowState::AwaitingConfirmation, FlowState::Responding) => true,
            (FlowState::AwaitingConfirmation, FlowState::HandoffPending) => true,
            (FlowState::AwaitingConfirmation, FlowState::Listening) => true,
            (FlowState::HandoffPending, FlowState::HandoffActive) => true,
            (FlowState::HandoffPending, FlowState::Error) => true,
            (FlowState::HandoffActive, FlowState::Listening) => true,
            (FlowState::HandoffActive, FlowState::Completed) => true,
            (FlowState::Error, FlowState::Listening) => true,
            (FlowState::Completed, FlowState::Idle) => true,
            _ => false,
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, FlowState::Completed | FlowState::Idle)
    }

    pub fn requires_user_input(&self) -> bool {
        matches!(
            self,
            FlowState::AwaitingConfirmation | FlowState::HandoffActive | FlowState::Listening
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowTransition {
    pub from: FlowState,
    pub to: FlowState,
    pub trigger: TransitionTrigger,
    pub guard: Option<String>,
    pub action: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransitionTrigger {
    UserMessage,
    ProcessingComplete,
    ConfirmationReceived,
    ConfirmationRejected,
    HandoffRequested,
    HandoffComplete,
    Error,
    Timeout,
    ManualOverride,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowEvent {
    pub id: String,
    pub event_type: EventType,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub from_state: Option<FlowState>,
    pub to_state: FlowState,
    pub data: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    StateEntered,
    StateExited,
    Transition,
    UserInput,
    AgentOutput,
    Error,
    Handoff,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationFlow {
    pub id: String,
    pub name: String,
    pub current_state: FlowState,
    pub states: Vec<FlowState>,
    pub transitions: Vec<FlowTransition>,
    pub event_history: Vec<FlowEvent>,
    pub context: FlowContext,
    pub config: FlowConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowContext {
    pub session_id: String,
    pub turn_count: usize,
    pub last_user_message: Option<String>,
    pub last_agent_message: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl FlowContext {
    pub fn new(session_id: &str) -> Self {
        Self {
            session_id: session_id.to_string(),
            turn_count: 0,
            last_user_message: None,
            last_agent_message: None,
            metadata: HashMap::new(),
        }
    }

    pub fn increment_turn(&mut self) {
        self.turn_count += 1;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowConfig {
    pub max_turns: usize,
    pub timeout_seconds: u64,
    pub auto_handoff_threshold: usize,
    pub confirmation_required: bool,
    pub show_confidence: bool,
}

impl Default for FlowConfig {
    fn default() -> Self {
        Self {
            max_turns: 50,
            timeout_seconds: 300,
            auto_handoff_threshold: 3,
            confirmation_required: false,
            show_confidence: true,
        }
    }
}

impl ConversationFlow {
    pub fn new(id: &str, name: &str) -> Self {
        let transitions = Self::default_transitions();
        let states = transitions.iter()
            .flat_map(|t| vec![t.from.clone(), t.to.clone()])
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        Self {
            id: id.to_string(),
            name: name.to_string(),
            current_state: FlowState::Idle,
            states,
            transitions,
            event_history: Vec::new(),
            context: FlowContext::new(id),
            config: FlowConfig::default(),
        }
    }

    fn default_transitions() -> Vec<FlowTransition> {
        vec![
            FlowTransition {
                from: FlowState::Idle,
                to: FlowState::Listening,
                trigger: TransitionTrigger::UserMessage,
                guard: None,
                action: None,
            },
            FlowTransition {
                from: FlowState::Listening,
                to: FlowState::Processing,
                trigger: TransitionTrigger::ProcessingComplete,
                guard: None,
                action: Some("process_input".to_string()),
            },
            FlowTransition {
                from: FlowState::Processing,
                to: FlowState::Responding,
                trigger: TransitionTrigger::ProcessingComplete,
                guard: Some("confidence >= 0.7".to_string()),
                action: Some("generate_response".to_string()),
            },
            FlowTransition {
                from: FlowState::Processing,
                to: FlowState::AwaitingConfirmation,
                trigger: TransitionTrigger::ProcessingComplete,
                guard: Some("confidence < 0.7 || requires_confirmation".to_string()),
                action: Some("request_confirmation".to_string()),
            },
            FlowTransition {
                from: FlowState::Processing,
                to: FlowState::HandoffPending,
                trigger: TransitionTrigger::HandoffRequested,
                guard: Some("max_retries_exceeded".to_string()),
                action: Some("initiate_handoff".to_string()),
            },
            FlowTransition {
                from: FlowState::AwaitingConfirmation,
                to: FlowState::Responding,
                trigger: TransitionTrigger::ConfirmationReceived,
                guard: None,
                action: Some("confirm_and_respond".to_string()),
            },
            FlowTransition {
                from: FlowState::AwaitingConfirmation,
                to: FlowState::Listening,
                trigger: TransitionTrigger::ConfirmationRejected,
                guard: None,
                action: Some("retry".to_string()),
            },
            FlowTransition {
                from: FlowState::AwaitingConfirmation,
                to: FlowState::HandoffPending,
                trigger: TransitionTrigger::HandoffRequested,
                guard: None,
                action: Some("initiate_handoff".to_string()),
            },
            FlowTransition {
                from: FlowState::HandoffPending,
                to: FlowState::HandoffActive,
                trigger: TransitionTrigger::HandoffRequested,
                guard: None,
                action: None,
            },
            FlowTransition {
                from: FlowState::HandoffActive,
                to: FlowState::Listening,
                trigger: TransitionTrigger::HandoffComplete,
                guard: None,
                action: Some("resume_conversation".to_string()),
            },
            FlowTransition {
                from: FlowState::Responding,
                to: FlowState::Listening,
                trigger: TransitionTrigger::UserMessage,
                guard: None,
                action: None,
            },
            FlowTransition {
                from: FlowState::Responding,
                to: FlowState::Completed,
                trigger: TransitionTrigger::ManualOverride,
                guard: Some("task_completed".to_string()),
                action: None,
            },
            FlowTransition {
                from: FlowState::Processing,
                to: FlowState::Error,
                trigger: TransitionTrigger::Error,
                guard: None,
                action: Some("log_error".to_string()),
            },
            FlowTransition {
                from: FlowState::Error,
                to: FlowState::Listening,
                trigger: TransitionTrigger::ManualOverride,
                guard: None,
                action: Some("recover".to_string()),
            },
        ]
    }

    pub fn transition(&mut self, trigger: TransitionTrigger) -> anyhow::Result<()> {
        let current_state = self.current_state.clone();
        
        let transition = self.transitions.iter()
            .find(|t| t.from == current_state && t.trigger == trigger);

        let transition = match transition {
            Some(t) => t,
            None => {
                return Err(anyhow::anyhow!(
                    "No valid transition from {:?} with trigger {:?}",
                    current_state,
                    trigger
                ));
            }
        };

        if !current_state.can_transition_to(&transition.to) {
            return Err(anyhow::anyhow!(
                "Invalid transition from {:?} to {:?}",
                current_state,
                transition.to
            ));
        }

        let event = FlowEvent {
            id: uuid::Uuid::new_v4().to_string(),
            event_type: EventType::Transition,
            timestamp: chrono::Utc::now(),
            from_state: Some(current_state),
            to_state: transition.to.clone(),
            data: HashMap::new(),
        };

        self.event_history.push(event);
        self.current_state = transition.to;

        Ok(())
    }

    pub fn handle_user_message(&mut self, message: &str) -> anyhow::Result<()> {
        if self.current_state == FlowState::Idle {
            self.transition(TransitionTrigger::UserMessage)?;
        }
        
        self.context.last_user_message = Some(message.to_string());
        self.context.increment_turn();

        if self.context.turn_count > self.config.max_turns {
            return Err(anyhow::anyhow!("Maximum turn count exceeded"));
        }

        self.transition(TransitionTrigger::ProcessingComplete)?;
        Ok(())
    }

    pub fn handle_agent_response(&mut self) -> anyhow::Result<()> {
        self.transition(TransitionTrigger::ProcessingComplete)
    }

    pub fn is_waiting_for_user(&self) -> bool {
        self.current_state.requires_user_input()
    }

    pub fn get_state_display(&self) -> StateDisplay {
        StateDisplay {
            state: format!("{:?}", self.current_state),
            state_name: self.current_state_name(),
            can_accept_input: self.is_waiting_for_user(),
            turn_count: self.context.turn_count,
            show_confidence: self.config.show_confidence,
            requires_confirmation: self.current_state == FlowState::AwaitingConfirmation,
        }
    }

    fn current_state_name(&self) -> String {
        match self.current_state {
            FlowState::Idle => "Ready",
            FlowState::Listening => "Listening...",
            FlowState::Processing => "Thinking...",
            FlowState::Responding => "Responding...",
            FlowState::AwaitingConfirmation => "Please Confirm",
            FlowState::HandoffPending => "Connecting to Agent...",
            FlowState::HandoffActive => "Human Agent",
            FlowState::Error => "Error",
            FlowState::Completed => "Done",
        }.to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateDisplay {
    pub state: String,
    pub state_name: String,
    pub can_accept_input: bool,
    pub turn_count: usize,
    pub show_confidence: bool,
    pub requires_confirmation: bool,
}

pub struct FlowBuilder {
    flow: ConversationFlow,
}

impl FlowBuilder {
    pub fn new(id: &str, name: &str) -> Self {
        Self {
            flow: ConversationFlow::new(id, name),
        }
    }

    pub fn with_config(mut self, config: FlowConfig) -> Self {
        self.flow.config = config;
        self
    }

    pub fn with_max_turns(mut self, max: usize) -> Self {
        self.flow.config.max_turns = max;
        self
    }

    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.flow.config.timeout_seconds = seconds;
        self
    }

    pub fn with_auto_handoff(mut self, threshold: usize) -> Self {
        self.flow.config.auto_handoff_threshold = threshold;
        self
    }

    pub fn require_confirmation(mut self, required: bool) -> Self {
        self.flow.config.confirmation_required = required;
        self
    }

    pub fn show_confidence(mut self, show: bool) -> Self {
        self.flow.config.show_confidence = show;
        self
    }

    pub fn add_transition(mut self, transition: FlowTransition) -> Self {
        self.flow.transitions.push(transition);
        self
    }

    pub fn build(self) -> ConversationFlow {
        self.flow
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_transitions() {
        assert!(FlowState::Idle.can_transition_to(&FlowState::Listening));
        assert!(!FlowState::Idle.can_transition_to(&FlowState::Completed));
    }

    #[test]
    fn test_flow_lifecycle() {
        let mut flow = FlowBuilder::new("test", "Test Flow")
            .with_max_turns(10)
            .build();

        assert_eq!(flow.current_state, FlowState::Idle);

        flow.handle_user_message("Hello").unwrap();
        assert_eq!(flow.current_state, FlowState::Processing);
    }

    #[test]
    fn test_flow_context() {
        let mut context = FlowContext::new("session_1");
        context.increment_turn();
        context.increment_turn();
        
        assert_eq!(context.turn_count, 2);
    }
}
