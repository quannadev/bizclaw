//! Notification system — routes messages to the best available channel.
//! PicoClaw-inspired: no queues, no Redis. Just pick a channel and send.

use serde::{Deserialize, Serialize};

/// A notification to send to the user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    /// Title/summary.
    pub title: String,
    /// Body content.
    pub body: String,
    /// Priority: low, normal, high, urgent.
    pub priority: NotifyPriority,
    /// Source (which task/event triggered this).
    pub source: String,
    /// Timestamp.
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Notification priority.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NotifyPriority {
    Low,
    Normal,
    High,
    Urgent,
}

/// Available notification channels.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotifyChannel {
    /// Channel type (telegram, discord, email, webhook, dashboard).
    pub channel_type: String,
    /// Whether this channel is configured and available.
    pub available: bool,
    /// Priority order (lower = preferred).
    pub priority: u8,
}

/// Notification router — picks the best channel to reach the user.
pub struct NotifyRouter {
    channels: Vec<NotifyChannel>,
    /// Notification history (in-memory ring buffer, max 100).
    history: Vec<Notification>,
}

impl NotifyRouter {
    pub fn new() -> Self {
        Self {
            channels: Vec::new(),
            history: Vec::new(),
        }
    }

    /// Register an available notification channel.
    pub fn register_channel(&mut self, channel_type: &str, priority: u8) {
        self.channels.push(NotifyChannel {
            channel_type: channel_type.to_string(),
            available: true,
            priority,
        });
        // Sort by priority (lowest number = highest priority)
        self.channels.sort_by_key(|c| c.priority);
    }

    /// Get the best available channel for a notification.
    pub fn best_channel(&self) -> Option<&NotifyChannel> {
        self.channels.iter().find(|c| c.available)
    }

    /// Get all available channels.
    pub fn available_channels(&self) -> Vec<&NotifyChannel> {
        self.channels.iter().filter(|c| c.available).collect()
    }

    /// Record a sent notification in history.
    pub fn record(&mut self, notification: Notification) {
        self.history.push(notification);
        // Ring buffer — keep last 100
        if self.history.len() > 100 {
            self.history.remove(0);
        }
    }

    /// Get notification history.
    pub fn history(&self) -> &[Notification] {
        &self.history
    }

    /// Create a notification.
    pub fn create(title: &str, body: &str, source: &str, priority: NotifyPriority) -> Notification {
        Notification {
            title: title.to_string(),
            body: body.to_string(),
            priority,
            source: source.to_string(),
            timestamp: chrono::Utc::now(),
        }
    }
}

impl Default for NotifyRouter {
    fn default() -> Self {
        Self::new()
    }
}
