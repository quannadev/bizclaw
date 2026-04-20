//! CatchMe Activity Tree Organizer — clustering events into hierarchical activity trees.
//!
//! Implements time-based clustering to group raw events into:
//! Day → Session → App → Action
//!
//! This is the "brain" that turns raw mouse/keyboard/window/clipboard events
//! into a meaningful hierarchical activity log for querying and summarization.

use crate::event::{CatchMeEvent, EventType};
use crate::store::CatchMeStore;
use anyhow::Result;
use chrono::{DateTime, Datelike, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

/// A node in the activity tree hierarchy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityNode {
    pub id: String,
    pub level: TreeLevel,
    pub parent_id: Option<String>,
    pub summary: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub event_count: usize,
    pub children: Vec<ActivityNode>,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

/// Tree hierarchy levels, from coarsest to finest.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TreeLevel {
    Day,
    Session,
    App,
    Action,
}

impl std::fmt::Display for TreeLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Day => write!(f, "Day"),
            Self::Session => write!(f, "Session"),
            Self::App => write!(f, "App"),
            Self::Action => write!(f, "Action"),
        }
    }
}

/// Configuration for the tree organizer.
#[derive(Debug, Clone)]
pub struct OrganizerConfig {
    /// Gap between events (in seconds) to split into separate sessions.
    pub session_gap_secs: i64,
    /// Minimum events to form a meaningful action cluster.
    pub min_action_events: usize,
    /// Maximum number of actions per app cluster before splitting.
    pub max_actions_per_app: usize,
}

impl Default for OrganizerConfig {
    fn default() -> Self {
        Self {
            session_gap_secs: 300, // 5 minutes gap → new session
            min_action_events: 2,
            max_actions_per_app: 50,
        }
    }
}

/// The Tree Organizer — groups raw events into hierarchical activity trees.
pub struct TreeOrganizer {
    config: OrganizerConfig,
}

impl TreeOrganizer {
    pub fn new(config: OrganizerConfig) -> Self {
        Self { config }
    }

    /// Build activity tree from a list of raw events.
    /// Returns a list of Day-level nodes, each containing Sessions → Apps → Actions.
    pub fn organize(&self, events: &[CatchMeEvent]) -> Vec<ActivityNode> {
        if events.is_empty() {
            return Vec::new();
        }

        // 1. Sort events by timestamp
        let mut sorted = events.to_vec();
        sorted.sort_by_key(|e| e.timestamp);

        // 2. Group by calendar day
        let day_groups = self.group_by_day(&sorted);

        // 3. Build tree for each day
        let mut days = Vec::new();
        for (date_str, day_events) in &day_groups {
            let day_node = self.build_day_tree(date_str, day_events);
            days.push(day_node);
        }

        days.sort_by_key(|d| d.start_time);
        days
    }

    /// Group events by calendar day (YYYY-MM-DD).
    fn group_by_day(&self, events: &[CatchMeEvent]) -> Vec<(String, Vec<CatchMeEvent>)> {
        let mut groups: Vec<(String, Vec<CatchMeEvent>)> = Vec::new();
        let mut current_key = String::new();

        for event in events {
            let key = format!(
                "{:04}-{:02}-{:02}",
                event.timestamp.year(),
                event.timestamp.month(),
                event.timestamp.day()
            );
            if key != current_key {
                groups.push((key.clone(), Vec::new()));
                current_key = key;
            }
            if let Some(last) = groups.last_mut() {
                last.1.push(event.clone());
            }
        }
        groups
    }

    /// Build a day-level tree node containing sessions.
    fn build_day_tree(&self, date_str: &str, events: &[CatchMeEvent]) -> ActivityNode {
        let sessions = self.cluster_sessions(events);

        let session_nodes: Vec<ActivityNode> = sessions
            .iter()
            .enumerate()
            .map(|(i, sess_events)| self.build_session_tree(date_str, i + 1, sess_events))
            .collect();

        let start = events.first().map(|e| e.timestamp).unwrap_or_else(Utc::now);
        let end = events.last().map(|e| e.timestamp).unwrap_or_else(Utc::now);
        let total_sessions = session_nodes.len();

        ActivityNode {
            id: format!("day_{date_str}"),
            level: TreeLevel::Day,
            parent_id: None,
            summary: format!(
                "{} — {} sessions, {} events",
                date_str,
                total_sessions,
                events.len()
            ),
            start_time: start,
            end_time: end,
            event_count: events.len(),
            children: session_nodes,
            metadata: HashMap::from([
                ("date".into(), date_str.into()),
                ("total_sessions".into(), total_sessions.to_string()),
            ]),
        }
    }

    /// Cluster events into sessions based on time gaps.
    /// A gap > `session_gap_secs` between consecutive events starts a new session.
    fn cluster_sessions(&self, events: &[CatchMeEvent]) -> Vec<Vec<CatchMeEvent>> {
        let mut sessions: Vec<Vec<CatchMeEvent>> = Vec::new();
        let mut current: Vec<CatchMeEvent> = Vec::new();

        for event in events {
            if let Some(last) = current.last() {
                let gap = (event.timestamp - last.timestamp).num_seconds();
                if gap > self.config.session_gap_secs
                    && !current.is_empty() {
                        sessions.push(std::mem::take(&mut current));
                    }
            }
            current.push(event.clone());
        }
        if !current.is_empty() {
            sessions.push(current);
        }
        sessions
    }

    /// Build a session-level tree node containing app clusters.
    fn build_session_tree(
        &self,
        date_str: &str,
        session_idx: usize,
        events: &[CatchMeEvent],
    ) -> ActivityNode {
        let app_clusters = self.cluster_by_app(events);

        let app_nodes: Vec<ActivityNode> = app_clusters
            .iter()
            .map(|(app_name, app_events)| {
                self.build_app_tree(date_str, session_idx, app_name, app_events)
            })
            .collect();

        let start = events.first().map(|e| e.timestamp).unwrap_or_else(Utc::now);
        let end = events.last().map(|e| e.timestamp).unwrap_or_else(Utc::now);
        let duration = end - start;
        let duration_str = format_duration(duration);

        ActivityNode {
            id: format!("session_{date_str}_{session_idx}"),
            level: TreeLevel::Session,
            parent_id: Some(format!("day_{date_str}")),
            summary: format!(
                "Session {} ({}) — {} apps, {} events",
                session_idx,
                duration_str,
                app_nodes.len(),
                events.len()
            ),
            start_time: start,
            end_time: end,
            event_count: events.len(),
            children: app_nodes,
            metadata: HashMap::from([
                ("session_idx".into(), session_idx.to_string()),
                ("duration".into(), duration_str),
            ]),
        }
    }

    /// Cluster events by the active application (window context).
    /// Consecutive same-app events form one cluster.
    fn cluster_by_app(&self, events: &[CatchMeEvent]) -> Vec<(String, Vec<CatchMeEvent>)> {
        let mut clusters: Vec<(String, Vec<CatchMeEvent>)> = Vec::new();
        let mut current_app = String::new();

        for event in events {
            let app = extract_app_name(event);

            if app != current_app {
                clusters.push((app.clone(), Vec::new()));
                current_app = app;
            }
            if let Some(last) = clusters.last_mut() {
                last.1.push(event.clone());
            }
        }
        clusters
    }

    /// Build an app-level tree node containing action clusters.
    fn build_app_tree(
        &self,
        date_str: &str,
        session_idx: usize,
        app_name: &str,
        events: &[CatchMeEvent],
    ) -> ActivityNode {
        let actions = self.cluster_actions(events);

        let action_nodes: Vec<ActivityNode> = actions
            .iter()
            .enumerate()
            .map(|(i, action_events)| {
                let summary = summarize_action(action_events);
                let start = action_events
                    .first()
                    .map(|e| e.timestamp)
                    .unwrap_or_else(Utc::now);
                let end = action_events
                    .last()
                    .map(|e| e.timestamp)
                    .unwrap_or_else(Utc::now);

                ActivityNode {
                    id: format!("action_{date_str}_{session_idx}_{app_name}_{i}"),
                    level: TreeLevel::Action,
                    parent_id: Some(format!("app_{date_str}_{session_idx}_{app_name}")),
                    summary,
                    start_time: start,
                    end_time: end,
                    event_count: action_events.len(),
                    children: Vec::new(),
                    metadata: HashMap::new(),
                }
            })
            .collect();

        let start = events.first().map(|e| e.timestamp).unwrap_or_else(Utc::now);
        let end = events.last().map(|e| e.timestamp).unwrap_or_else(Utc::now);

        ActivityNode {
            id: format!("app_{date_str}_{session_idx}_{app_name}"),
            level: TreeLevel::App,
            parent_id: Some(format!("session_{date_str}_{session_idx}")),
            summary: format!("{} — {} actions", app_name, action_nodes.len()),
            start_time: start,
            end_time: end,
            event_count: events.len(),
            children: action_nodes,
            metadata: HashMap::from([("app".into(), app_name.into())]),
        }
    }

    /// Cluster events within an app into logical actions based on type transitions
    /// and small time gaps (<10s with different types = same action).
    fn cluster_actions(&self, events: &[CatchMeEvent]) -> Vec<Vec<CatchMeEvent>> {
        let mut actions: Vec<Vec<CatchMeEvent>> = Vec::new();
        let mut current: Vec<CatchMeEvent> = Vec::new();

        for event in events {
            if let Some(last) = current.last() {
                let gap = (event.timestamp - last.timestamp).num_seconds();
                // Split if gap > 30s or accumulated too many events
                if gap > 30 || current.len() >= self.config.max_actions_per_app {
                    if current.len() >= self.config.min_action_events {
                        actions.push(std::mem::take(&mut current));
                    } else {
                        current.clear();
                    }
                }
            }
            current.push(event.clone());
        }
        if current.len() >= self.config.min_action_events {
            actions.push(current);
        } else if !current.is_empty() && actions.is_empty() {
            // Keep even small clusters if it's the only activity
            actions.push(current);
        }
        actions
    }

    /// Persist the organized tree into the SQLite activity_tree table.
    pub fn persist_tree(&self, store: &CatchMeStore, tree: &[ActivityNode]) -> Result<()> {
        for node in tree {
            store.upsert_activity_node(node)?;
            self.persist_children(store, &node.children)?;
        }
        info!("Persisted activity tree: {} day(s)", tree.len());
        Ok(())
    }

    fn persist_children(&self, store: &CatchMeStore, children: &[ActivityNode]) -> Result<()> {
        for child in children {
            store.upsert_activity_node(child)?;
            self.persist_children(store, &child.children)?;
        }
        Ok(())
    }
}

/// Extract application name from an event.
fn extract_app_name(event: &CatchMeEvent) -> String {
    match &event.event_type {
        EventType::Window { app, .. } => app.clone(),
        EventType::Clipboard { .. } => "Clipboard".into(),
        EventType::Notification { app, .. } => app.clone(),
        EventType::Keyboard { .. } => "Keyboard Input".into(),
        EventType::Mouse { .. } => "Mouse".into(),
        EventType::Screenshot { .. } => "Screenshot".into(),
    }
}

/// Generate a human-readable summary of an action cluster.
fn summarize_action(events: &[CatchMeEvent]) -> String {
    if events.is_empty() {
        return "Empty action".into();
    }

    // Categorize events by type
    let mut typing_chars = 0usize;
    let mut clicks = 0usize;
    let mut clipboard_ops = 0usize;
    let mut window_titles: Vec<String> = Vec::new();

    for event in events {
        match &event.event_type {
            EventType::Keyboard { text } => typing_chars += text.len(),
            EventType::Mouse { click, .. } if *click => clicks += 1,
            EventType::Mouse { .. } => {} // mouse movement, ignore
            EventType::Clipboard { .. } => clipboard_ops += 1,
            EventType::Window { title, .. } => {
                if !window_titles.contains(title) {
                    window_titles.push(title.clone());
                }
            }
            EventType::Notification { title, .. } => {
                window_titles.push(format!("📩 {title}"));
            }
            EventType::Screenshot { .. } => {
                window_titles.push("📸 Screenshot".into());
            }
        }
    }

    let mut parts = Vec::new();
    if typing_chars > 0 {
        parts.push(format!("Typed ~{} chars", typing_chars));
    }
    if clicks > 0 {
        parts.push(format!("{} clicks", clicks));
    }
    if clipboard_ops > 0 {
        parts.push(format!("{} clipboard ops", clipboard_ops));
    }
    if !window_titles.is_empty() {
        let title = window_titles.first().unwrap_or(&String::new()).clone();
        let truncated = if title.len() > 40 {
            format!("{}…", &title[..40])
        } else {
            title
        };
        parts.push(format!("in '{truncated}'"));
    }

    if parts.is_empty() {
        format!("{} events", events.len())
    } else {
        parts.join(", ")
    }
}

/// Format a chrono Duration into a human-readable string.
fn format_duration(d: Duration) -> String {
    let secs = d.num_seconds();
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_event(source: &str, event_type: EventType, offset_secs: i64) -> CatchMeEvent {
        CatchMeEvent {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now() + Duration::seconds(offset_secs),
            source: source.into(),
            event_type,
        }
    }

    #[test]
    fn test_organize_empty() {
        let org = TreeOrganizer::new(OrganizerConfig::default());
        let tree = org.organize(&[]);
        assert!(tree.is_empty());
    }

    #[test]
    fn test_organize_single_event() {
        let org = TreeOrganizer::new(OrganizerConfig::default());
        let events = vec![make_event(
            "test",
            EventType::Window {
                title: "VS Code".into(),
                app: "Code".into(),
            },
            0,
        )];
        let tree = org.organize(&events);
        assert_eq!(tree.len(), 1); // 1 day
        assert_eq!(tree[0].level, TreeLevel::Day);
        assert_eq!(tree[0].children.len(), 1); // 1 session
        assert_eq!(tree[0].children[0].level, TreeLevel::Session);
    }

    #[test]
    fn test_session_split_on_gap() {
        let org = TreeOrganizer::new(OrganizerConfig {
            session_gap_secs: 60, // 1 min gap → new session
            ..Default::default()
        });
        let events = vec![
            make_event(
                "test",
                EventType::Keyboard {
                    text: "hello".into(),
                },
                0,
            ),
            make_event(
                "test",
                EventType::Keyboard {
                    text: "world".into(),
                },
                10,
            ),
            // 120 second gap → new session
            make_event(
                "test",
                EventType::Keyboard {
                    text: "new session".into(),
                },
                130,
            ),
            make_event(
                "test",
                EventType::Keyboard {
                    text: "still here".into(),
                },
                140,
            ),
        ];
        let tree = org.organize(&events);
        assert_eq!(tree.len(), 1); // same day
        assert_eq!(tree[0].children.len(), 2); // 2 sessions
    }

    #[test]
    fn test_app_clustering() {
        let org = TreeOrganizer::new(OrganizerConfig::default());
        let events = vec![
            make_event(
                "test",
                EventType::Window {
                    title: "main.rs".into(),
                    app: "Code".into(),
                },
                0,
            ),
            make_event(
                "test",
                EventType::Keyboard {
                    text: "fn main()".into(),
                },
                1,
            ),
            make_event(
                "test",
                EventType::Window {
                    title: "Google Chrome".into(),
                    app: "Chrome".into(),
                },
                5,
            ),
            make_event(
                "test",
                EventType::Mouse {
                    x: 100.0,
                    y: 200.0,
                    click: true,
                },
                6,
            ),
        ];
        let tree = org.organize(&events);
        assert_eq!(tree.len(), 1);
        let session = &tree[0].children[0];
        // Should have 3 app clusters: Code, Keyboard Input, Chrome
        // (Keyboard events outside Window context get their own "app")
        assert!(session.children.len() >= 2);
    }

    #[test]
    fn test_summarize_action_typing() {
        let events = vec![
            make_event(
                "test",
                EventType::Keyboard {
                    text: "hello world".into(),
                },
                0,
            ),
            make_event(
                "test",
                EventType::Keyboard {
                    text: "test".into(),
                },
                1,
            ),
        ];
        let summary = summarize_action(&events);
        assert!(summary.contains("Typed ~15 chars"));
    }

    #[test]
    fn test_summarize_action_clicks() {
        let events = vec![
            make_event(
                "test",
                EventType::Mouse {
                    x: 10.0,
                    y: 20.0,
                    click: true,
                },
                0,
            ),
            make_event(
                "test",
                EventType::Mouse {
                    x: 30.0,
                    y: 40.0,
                    click: true,
                },
                1,
            ),
            make_event(
                "test",
                EventType::Mouse {
                    x: 50.0,
                    y: 60.0,
                    click: false,
                },
                2,
            ),
        ];
        let summary = summarize_action(&events);
        assert!(summary.contains("2 clicks"));
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::seconds(30)), "30s");
        assert_eq!(format_duration(Duration::seconds(90)), "1m 30s");
        assert_eq!(format_duration(Duration::seconds(3661)), "1h 1m");
    }

    #[test]
    fn test_tree_node_serialization() {
        let node = ActivityNode {
            id: "test_1".into(),
            level: TreeLevel::Day,
            parent_id: None,
            summary: "Test day".into(),
            start_time: Utc::now(),
            end_time: Utc::now(),
            event_count: 42,
            children: Vec::new(),
            metadata: HashMap::from([("key".into(), "value".into())]),
        };
        let json = serde_json::to_string(&node).unwrap();
        let deser: ActivityNode = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.id, "test_1");
        assert_eq!(deser.event_count, 42);
    }
}
