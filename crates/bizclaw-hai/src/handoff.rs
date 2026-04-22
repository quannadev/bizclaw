//! Human-agent handoff management

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandoffTrigger {
    UserRequest,
    ConfidenceLow,
    Error,
    Timeout,
    ManualEscalation,
    MaxRetriesExceeded,
    SensitiveTopic,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffContext {
    pub session_id: String,
    pub conversation_history: Vec<ConversationTurn>,
    pub user_info: Option<UserInfo>,
    pub agent_summary: String,
    pub escalation_reason: String,
    pub priority: HandoffPriority,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationTurn {
    pub role: String,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: String,
    pub name: String,
    pub email: Option<String>,
    pub tier: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandoffPriority {
    Low,
    Normal,
    High,
    Urgent,
    Critical,
}

impl HandoffPriority {
    pub fn sla_minutes(&self) -> u64 {
        match self {
            HandoffPriority::Low => 480,
            HandoffPriority::Normal => 60,
            HandoffPriority::High => 30,
            HandoffPriority::Urgent => 15,
            HandoffPriority::Critical => 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffRequest {
    pub id: String,
    pub trigger: HandoffTrigger,
    pub context: HandoffContext,
    pub requested_at: chrono::DateTime<chrono::Utc>,
    pub status: HandoffStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandoffStatus {
    Pending,
    InProgress,
    Accepted,
    Declined,
    Completed,
    Cancelled,
    Timeout,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffResult {
    pub request_id: String,
    pub status: HandoffStatus,
    pub human_agent_id: Option<String>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub notes: Vec<String>,
    pub satisfaction_rating: Option<u8>,
}

pub struct HandoffManager {
    active_handlers: HashMap<String, String>,
    queue: Vec<HandoffRequest>,
    max_queue_size: usize,
}

impl Default for HandoffManager {
    fn default() -> Self {
        Self::new()
    }
}

impl HandoffManager {
    pub fn new() -> Self {
        Self {
            active_handlers: HashMap::new(),
            queue: Vec::new(),
            max_queue_size: 100,
        }
    }

    pub fn create_request(
        &mut self,
        trigger: HandoffTrigger,
        context: HandoffContext,
    ) -> HandoffRequest {
        let priority = self.determine_priority(&trigger, &context);
        
        let mut request = HandoffRequest {
            id: uuid::Uuid::new_v4().to_string(),
            trigger,
            context,
            requested_at: chrono::Utc::now(),
            status: HandoffStatus::Pending,
        };

        request.context.priority = priority;

        if self.queue.len() < self.max_queue_size {
            self.queue.push(request.clone());
        }

        self.sort_queue_by_priority();

        request
    }

    fn determine_priority(&self, trigger: &HandoffTrigger, context: &HandoffContext) -> HandoffPriority {
        if let Some(user_tier) = &context.user_info {
            match user_tier.tier.as_str() {
                "premium" => return HandoffPriority::High,
                "enterprise" => return HandoffPriority::Urgent,
                _ => {}
            }
        }

        match trigger {
            HandoffTrigger::Critical => HandoffPriority::Critical,
            HandoffTrigger::UserRequest => HandoffPriority::Normal,
            HandoffTrigger::ConfidenceLow => HandoffPriority::Normal,
            HandoffTrigger::Error => HandoffPriority::High,
            HandoffTrigger::Timeout => HandoffPriority::Normal,
            HandoffTrigger::ManualEscalation => HandoffPriority::High,
            HandoffTrigger::MaxRetriesExceeded => HandoffPriority::High,
            HandoffTrigger::SensitiveTopic => HandoffPriority::Urgent,
        }
    }

    fn sort_queue_by_priority(&mut self) {
        self.queue.sort_by(|a, b| {
            let a_prio = match a.context.priority {
                HandoffPriority::Critical => 5,
                HandoffPriority::Urgent => 4,
                HandoffPriority::High => 3,
                HandoffPriority::Normal => 2,
                HandoffPriority::Low => 1,
            };
            let b_prio = match b.context.priority {
                HandoffPriority::Critical => 5,
                HandoffPriority::Urgent => 4,
                HandoffPriority::High => 3,
                HandoffPriority::Normal => 2,
                HandoffPriority::Low => 1,
            };
            b_prio.cmp(&a_prio)
        });
    }

    pub fn accept_request(&mut self, request_id: &str, handler_id: &str) -> Option<HandoffResult> {
        if let Some(pos) = self.queue.iter().position(|r| r.id == request_id) {
            let request = &mut self.queue[pos];
            request.status = HandoffStatus::Accepted;
            
            self.active_handlers.insert(request_id.to_string(), handler_id.to_string());

            Some(HandoffResult {
                request_id: request_id.to_string(),
                status: HandoffStatus::InProgress,
                human_agent_id: Some(handler_id.to_string()),
                started_at: Some(chrono::Utc::now()),
                completed_at: None,
                notes: Vec::new(),
                satisfaction_rating: None,
            })
        } else {
            None
        }
    }

    pub fn complete_request(&mut self, request_id: &str, notes: Vec<String>) -> Option<HandoffResult> {
        if let Some(handler_id) = self.active_handlers.remove(request_id) {
            self.queue.retain(|r| r.id != request_id);

            Some(HandoffResult {
                request_id: request_id.to_string(),
                status: HandoffStatus::Completed,
                human_agent_id: Some(handler_id),
                started_at: None,
                completed_at: Some(chrono::Utc::now()),
                notes,
                satisfaction_rating: None,
            })
        } else {
            None
        }
    }

    pub fn get_pending_requests(&self) -> Vec<&HandoffRequest> {
        self.queue.iter()
            .filter(|r| r.status == HandoffStatus::Pending)
            .collect()
    }

    pub fn get_queue_length(&self) -> usize {
        self.queue.len()
    }

    pub fn generate_context_summary(&self, context: &HandoffContext) -> String {
        let mut summary = String::new();

        summary.push_str(&format!(
            "Session: {}\n",
            context.session_id
        ));

        if let Some(user) = &context.user_info {
            summary.push_str(&format!(
                "User: {} ({})\n",
                user.name, user.tier
            ));
        }

        summary.push_str(&format!(
            "Escalation Reason: {}\n",
            context.escalation_reason
        ));

        summary.push_str(&format!(
            "Priority: {:?}\n",
            context.priority
        ));

        if !context.conversation_history.is_empty() {
            summary.push_str(&format!(
                "\nRecent conversation ({} turns):\n",
                context.conversation_history.len()
            ));

            for (i, turn) in context.conversation_history.iter().rev().take(5).enumerate() {
                summary.push_str(&format!(
                    "{}: [{}] {}\n",
                    i + 1,
                    turn.role,
                    &turn.content[..turn.content.len().min(100)]
                ));
            }
        }

        summary
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handoff_priority_sla() {
        assert_eq!(HandoffPriority::Critical.sla_minutes(), 5);
        assert_eq!(HandoffPriority::Normal.sla_minutes(), 60);
        assert_eq!(HandoffPriority::Low.sla_minutes(), 480);
    }

    #[test]
    fn test_handoff_request_creation() {
        let mut manager = HandoffManager::new();
        
        let context = HandoffContext {
            session_id: "test_session".to_string(),
            conversation_history: Vec::new(),
            user_info: None,
            agent_summary: "User asked about billing".to_string(),
            escalation_reason: "Low confidence".to_string(),
            priority: HandoffPriority::Normal,
            metadata: HashMap::new(),
        };

        let request = manager.create_request(HandoffTrigger::ConfidenceLow, context);
        
        assert_eq!(request.status, HandoffStatus::Pending);
        assert_eq!(manager.get_queue_length(), 1);
    }

    #[test]
    fn test_priority_based_sorting() {
        let mut manager = HandoffManager::new();
        
        let mut context1 = HandoffContext {
            session_id: "s1".to_string(),
            conversation_history: Vec::new(),
            user_info: None,
            agent_summary: String::new(),
            escalation_reason: String::new(),
            priority: HandoffPriority::Low,
            metadata: HashMap::new(),
        };

        let mut context2 = HandoffContext {
            session_id: "s2".to_string(),
            conversation_history: Vec::new(),
            user_info: None,
            agent_summary: String::new(),
            escalation_reason: String::new(),
            priority: HandoffPriority::Critical,
            metadata: HashMap::new(),
        };

        manager.create_request(HandoffTrigger::UserRequest, context1);
        manager.create_request(HandoffTrigger::Critical, context2);

        let queue = manager.get_pending_requests();
        assert_eq!(queue[0].context.priority, HandoffPriority::Critical);
    }
}
