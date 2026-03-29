//! Ticket System — conversation-to-ticket mapping with audit trail.
//!
//! Maps each conversation (from Zalo, Telegram, etc.) to a ticket
//! with status tracking, agent assignment, cost accounting, and resolution metrics.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Ticket status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TicketStatus {
    /// New ticket, not yet assigned
    Open,
    /// Assigned to an agent
    Assigned,
    /// Agent is actively working on it
    InProgress,
    /// Escalated to lead/human
    Escalated,
    /// Waiting for customer response
    WaitingCustomer,
    /// Resolved
    Resolved,
    /// Closed (after resolution confirmation)
    Closed,
}

impl std::fmt::Display for TicketStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Open => write!(f, "📩 Open"),
            Self::Assigned => write!(f, "📋 Assigned"),
            Self::InProgress => write!(f, "🔄 In Progress"),
            Self::Escalated => write!(f, "🚨 Escalated"),
            Self::WaitingCustomer => write!(f, "⏳ Waiting Customer"),
            Self::Resolved => write!(f, "✅ Resolved"),
            Self::Closed => write!(f, "🔒 Closed"),
        }
    }
}

/// Priority level.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum Priority {
    Low,
    #[default]
    Normal,
    High,
    Urgent,
}

/// An event in the ticket audit trail.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketEvent {
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Event type
    pub event_type: TicketEventType,
    /// Who triggered this event
    pub actor: String,
    /// Optional details
    pub details: Option<String>,
}

/// Types of ticket events.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TicketEventType {
    /// Ticket created
    Created,
    /// Status changed
    StatusChanged {
        from: TicketStatus,
        to: TicketStatus,
    },
    /// Agent assigned
    AgentAssigned { agent_id: String },
    /// Message received from customer
    CustomerMessage,
    /// Agent replied
    AgentReply,
    /// Escalated
    Escalated { reason: String },
    /// Priority changed
    PriorityChanged { from: Priority, to: Priority },
    /// Note added
    NoteAdded,
    /// Token usage recorded
    TokenUsage { tokens: u64, cost_usd: f64 },
}

/// A support/conversation ticket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ticket {
    /// Unique ticket ID
    pub id: String,
    /// Channel of origin
    pub channel: String,
    /// Thread ID in the channel
    pub thread_id: String,
    /// Customer/sender ID
    pub customer_id: String,
    /// Customer name (if known)
    pub customer_name: Option<String>,
    /// Current status
    pub status: TicketStatus,
    /// Priority
    pub priority: Priority,
    /// Assigned agent ID
    pub assigned_agent: Option<String>,
    /// Subject/summary
    pub subject: String,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Created at
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last updated at
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// Resolved at
    pub resolved_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Audit trail
    pub events: Vec<TicketEvent>,
    /// Total tokens spent on this ticket
    pub total_tokens: u64,
    /// Total USD cost
    pub total_cost_usd: f64,
    /// Number of messages in the conversation
    pub message_count: u32,
}

impl Ticket {
    /// Create a new ticket from an incoming message.
    pub fn from_message(
        channel: &str,
        thread_id: &str,
        customer_id: &str,
        first_message: &str,
    ) -> Self {
        let now = chrono::Utc::now();
        let id = format!("TK-{}", &uuid::Uuid::new_v4().to_string()[..8]);

        // Auto-generate subject from first message
        let subject = if first_message.chars().count() > 80 {
            let t: String = first_message.chars().take(77).collect();
            format!("{}...", t)
        } else {
            first_message.to_string()
        };

        let mut ticket = Self {
            id: id.clone(),
            channel: channel.to_string(),
            thread_id: thread_id.to_string(),
            customer_id: customer_id.to_string(),
            customer_name: None,
            status: TicketStatus::Open,
            priority: Priority::Normal,
            assigned_agent: None,
            subject,
            tags: Vec::new(),
            created_at: now,
            updated_at: now,
            resolved_at: None,
            events: Vec::new(),
            total_tokens: 0,
            total_cost_usd: 0.0,
            message_count: 1,
        };

        ticket.add_event(
            TicketEventType::Created,
            "system",
            Some(format!("Ticket created from {} channel", channel)),
        );

        ticket
    }

    /// Add an event to the audit trail.
    pub fn add_event(&mut self, event_type: TicketEventType, actor: &str, details: Option<String>) {
        self.events.push(TicketEvent {
            timestamp: chrono::Utc::now(),
            event_type,
            actor: actor.to_string(),
            details,
        });
        self.updated_at = chrono::Utc::now();
    }

    /// Assign to an agent.
    pub fn assign(&mut self, agent_id: &str) {
        let old_status = self.status.clone();
        self.assigned_agent = Some(agent_id.to_string());
        self.status = TicketStatus::Assigned;
        self.add_event(
            TicketEventType::AgentAssigned {
                agent_id: agent_id.to_string(),
            },
            "system",
            None,
        );
        self.add_event(
            TicketEventType::StatusChanged {
                from: old_status,
                to: TicketStatus::Assigned,
            },
            "system",
            None,
        );
    }

    /// Escalate the ticket.
    pub fn escalate(&mut self, reason: &str) {
        let old_status = self.status.clone();
        self.status = TicketStatus::Escalated;
        self.add_event(
            TicketEventType::Escalated {
                reason: reason.to_string(),
            },
            "system",
            None,
        );
        self.add_event(
            TicketEventType::StatusChanged {
                from: old_status,
                to: TicketStatus::Escalated,
            },
            "system",
            None,
        );
    }

    /// Mark as resolved.
    pub fn resolve(&mut self, agent_id: &str) {
        let old_status = self.status.clone();
        self.status = TicketStatus::Resolved;
        self.resolved_at = Some(chrono::Utc::now());
        self.add_event(
            TicketEventType::StatusChanged {
                from: old_status,
                to: TicketStatus::Resolved,
            },
            agent_id,
            None,
        );
    }

    /// Record token usage for this ticket.
    pub fn record_tokens(&mut self, tokens: u64, cost_usd: f64) {
        self.total_tokens += tokens;
        self.total_cost_usd += cost_usd;
        self.add_event(
            TicketEventType::TokenUsage { tokens, cost_usd },
            "system",
            None,
        );
    }

    /// Get resolution time in seconds.
    pub fn resolution_time_seconds(&self) -> Option<i64> {
        self.resolved_at
            .map(|r| (r - self.created_at).num_seconds())
    }
}

/// Ticket manager — tracks all tickets.
pub struct TicketManager {
    tickets: Arc<RwLock<HashMap<String, Ticket>>>,
    /// Map thread_id → ticket_id for quick lookup
    thread_index: Arc<RwLock<HashMap<String, String>>>,
}

impl TicketManager {
    pub fn new() -> Self {
        Self {
            tickets: Arc::new(RwLock::new(HashMap::new())),
            thread_index: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new ticket from an incoming message.
    /// Returns the ticket ID.
    pub async fn create_ticket(
        &self,
        channel: &str,
        thread_id: &str,
        customer_id: &str,
        first_message: &str,
    ) -> String {
        let ticket = Ticket::from_message(channel, thread_id, customer_id, first_message);
        let id = ticket.id.clone();

        self.thread_index
            .write()
            .await
            .insert(thread_id.to_string(), id.clone());
        self.tickets.write().await.insert(id.clone(), ticket);

        tracing::info!(
            "Ticket: created {} for {} thread {}",
            id,
            channel,
            thread_id
        );
        id
    }

    /// Get or create ticket for a thread.
    pub async fn get_or_create(
        &self,
        channel: &str,
        thread_id: &str,
        customer_id: &str,
        message: &str,
    ) -> String {
        // Check if ticket exists for this thread
        if let Some(ticket_id) = self.thread_index.read().await.get(thread_id) {
            // Update message count
            if let Some(ticket) = self.tickets.write().await.get_mut(ticket_id) {
                ticket.message_count += 1;
                ticket.updated_at = chrono::Utc::now();
            }
            return ticket_id.clone();
        }

        // Create new ticket
        self.create_ticket(channel, thread_id, customer_id, message)
            .await
    }

    /// Get a ticket by ID.
    pub async fn get_ticket(&self, ticket_id: &str) -> Option<Ticket> {
        self.tickets.read().await.get(ticket_id).cloned()
    }

    /// Get ticket for a thread.
    pub async fn ticket_for_thread(&self, thread_id: &str) -> Option<Ticket> {
        let ticket_id = self.thread_index.read().await.get(thread_id)?.clone();
        self.get_ticket(&ticket_id).await
    }

    /// Update a ticket (mutate in place).
    pub async fn update<F>(&self, ticket_id: &str, updater: F) -> bool
    where
        F: FnOnce(&mut Ticket),
    {
        if let Some(ticket) = self.tickets.write().await.get_mut(ticket_id) {
            updater(ticket);
            true
        } else {
            false
        }
    }

    /// List tickets by status.
    pub async fn list_by_status(&self, status: &TicketStatus) -> Vec<Ticket> {
        self.tickets
            .read()
            .await
            .values()
            .filter(|t| &t.status == status)
            .cloned()
            .collect()
    }

    /// Get dashboard summary.
    pub async fn summary(&self) -> serde_json::Value {
        let tickets = self.tickets.read().await;
        let total = tickets.len();

        let by_status = |s: &TicketStatus| tickets.values().filter(|t| &t.status == s).count();

        let open = by_status(&TicketStatus::Open);
        let assigned = by_status(&TicketStatus::Assigned);
        let in_progress = by_status(&TicketStatus::InProgress);
        let escalated = by_status(&TicketStatus::Escalated);
        let resolved = by_status(&TicketStatus::Resolved);
        let closed = by_status(&TicketStatus::Closed);

        let total_tokens: u64 = tickets.values().map(|t| t.total_tokens).sum();
        let total_cost: f64 = tickets.values().map(|t| t.total_cost_usd).sum();

        let avg_resolution: f64 = {
            let resolved_tickets: Vec<_> = tickets
                .values()
                .filter_map(|t| t.resolution_time_seconds())
                .collect();
            if resolved_tickets.is_empty() {
                0.0
            } else {
                resolved_tickets.iter().sum::<i64>() as f64 / resolved_tickets.len() as f64
            }
        };

        // Recent tickets (last 10)
        let mut recent: Vec<&Ticket> = tickets.values().collect();
        recent.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        let recent_entries: Vec<serde_json::Value> = recent
            .iter()
            .take(10)
            .map(|t| {
                serde_json::json!({
                    "id": t.id,
                    "channel": t.channel,
                    "status": t.status.to_string(),
                    "subject": t.subject,
                    "customer_id": t.customer_id,
                    "agent": t.assigned_agent,
                    "messages": t.message_count,
                    "tokens": t.total_tokens,
                    "created": t.created_at.to_rfc3339(),
                })
            })
            .collect();

        serde_json::json!({
            "total": total,
            "open": open,
            "assigned": assigned,
            "in_progress": in_progress,
            "escalated": escalated,
            "resolved": resolved,
            "closed": closed,
            "total_tokens": total_tokens,
            "total_cost_usd": total_cost,
            "avg_resolution_seconds": avg_resolution,
            "recent": recent_entries,
        })
    }
}

impl Default for TicketManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ticket_creation() {
        let mgr = TicketManager::new();
        let id = mgr
            .create_ticket("zalo", "thread-123", "user-456", "Xin chào, tôi cần hỗ trợ")
            .await;

        assert!(id.starts_with("TK-"));

        let ticket = mgr.get_ticket(&id).await.unwrap();
        assert_eq!(ticket.status, TicketStatus::Open);
        assert_eq!(ticket.channel, "zalo");
        assert_eq!(ticket.message_count, 1);
    }

    #[tokio::test]
    async fn test_ticket_lifecycle() {
        let mgr = TicketManager::new();
        let id = mgr
            .create_ticket("zalo", "thread-123", "user-456", "Help me")
            .await;

        // Assign
        mgr.update(&id, |t| t.assign("agent-1")).await;
        let ticket = mgr.get_ticket(&id).await.unwrap();
        assert_eq!(ticket.status, TicketStatus::Assigned);

        // Record tokens
        mgr.update(&id, |t| t.record_tokens(500, 0.001)).await;
        let ticket = mgr.get_ticket(&id).await.unwrap();
        assert_eq!(ticket.total_tokens, 500);

        // Resolve
        mgr.update(&id, |t| t.resolve("agent-1")).await;
        let ticket = mgr.get_ticket(&id).await.unwrap();
        assert_eq!(ticket.status, TicketStatus::Resolved);
        assert!(ticket.resolved_at.is_some());
    }

    #[tokio::test]
    async fn test_get_or_create() {
        let mgr = TicketManager::new();

        // First message creates ticket
        let id1 = mgr
            .get_or_create("zalo", "thread-123", "user-456", "Hello")
            .await;

        // Same thread returns same ticket
        let id2 = mgr
            .get_or_create("zalo", "thread-123", "user-456", "Follow up")
            .await;

        assert_eq!(id1, id2);

        // Message count should be 2
        let ticket = mgr.get_ticket(&id1).await.unwrap();
        assert_eq!(ticket.message_count, 2);
    }

    #[tokio::test]
    async fn test_ticket_escalation() {
        let mgr = TicketManager::new();
        let id = mgr
            .create_ticket("telegram", "t-1", "u-1", "Urgent issue")
            .await;

        mgr.update(&id, |t| t.escalate("Customer is VIP")).await;
        let ticket = mgr.get_ticket(&id).await.unwrap();
        assert_eq!(ticket.status, TicketStatus::Escalated);
        // Should have 3 events: Created + Escalated + StatusChanged
        assert!(ticket.events.len() >= 3);
    }

    #[tokio::test]
    async fn test_ticket_token_recording() {
        let mgr = TicketManager::new();
        let id = mgr.create_ticket("zalo", "t-2", "u-2", "Test").await;

        mgr.update(&id, |t| {
            t.record_tokens(1000, 0.01);
            t.record_tokens(500, 0.005);
        })
        .await;

        let ticket = mgr.get_ticket(&id).await.unwrap();
        assert_eq!(ticket.total_tokens, 1500);
        assert!((ticket.total_cost_usd - 0.015).abs() < 0.0001);
    }

    #[tokio::test]
    async fn test_ticket_resolution_time() {
        let mgr = TicketManager::new();
        let id = mgr.create_ticket("messenger", "t-3", "u-3", "Help").await;

        mgr.update(&id, |t| t.resolve("agent-1")).await;
        let ticket = mgr.get_ticket(&id).await.unwrap();
        assert!(ticket.resolution_time_seconds().is_some());
        assert!(ticket.resolution_time_seconds().unwrap() >= 0);
    }

    #[tokio::test]
    async fn test_ticket_thread_lookup() {
        let mgr = TicketManager::new();
        let id = mgr
            .create_ticket("zalo", "thread-xyz", "u-1", "Hello")
            .await;

        let found = mgr.ticket_for_thread("thread-xyz").await;
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, id);

        let not_found = mgr.ticket_for_thread("nonexistent").await;
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_list_by_status() {
        let mgr = TicketManager::new();
        mgr.create_ticket("zalo", "t-1", "u-1", "A").await;
        mgr.create_ticket("zalo", "t-2", "u-2", "B").await;
        let id3 = mgr.create_ticket("zalo", "t-3", "u-3", "C").await;

        mgr.update(&id3, |t| t.resolve("agent-1")).await;

        let open = mgr.list_by_status(&TicketStatus::Open).await;
        assert_eq!(open.len(), 2);
        let resolved = mgr.list_by_status(&TicketStatus::Resolved).await;
        assert_eq!(resolved.len(), 1);
    }

    #[tokio::test]
    async fn test_ticket_summary() {
        let mgr = TicketManager::new();
        mgr.create_ticket("zalo", "t-1", "u-1", "Test ticket").await;

        let summary = mgr.summary().await;
        assert_eq!(summary["total"], 1);
        assert_eq!(summary["open"], 1);
    }
}
