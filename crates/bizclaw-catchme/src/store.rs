use crate::event::CatchMeEvent;
use anyhow::Result;
use rusqlite::Connection;
use std::path::Path;
use tracing::info;

pub struct CatchMeStore {
    conn: Connection,
}

impl CatchMeStore {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        info!("Initializing CatchMe SQLite store at {:?}", path.as_ref());
        let conn = Connection::open(path)?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS events (
                id TEXT PRIMARY KEY,
                timestamp TEXT NOT NULL,
                source TEXT NOT NULL,
                event_type TEXT NOT NULL,
                data JSON NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS activity_tree (
                id TEXT PRIMARY KEY,
                level TEXT NOT NULL,          -- Day, Session, App, Location, Action
                parent_id TEXT,
                summary TEXT NOT NULL,
                start_time TEXT NOT NULL,
                end_time TEXT NOT NULL
            )",
            [],
        )?;

        Ok(Self { conn })
    }

    pub fn insert_event(&self, event: &CatchMeEvent) -> Result<()> {
        let event_type_str = serde_json::to_string(&event.event_type)?;
        let data = serde_json::to_string(&event)?;
        self.conn.execute(
            "INSERT INTO events (id, timestamp, source, event_type, data) VALUES (?1, ?2, ?3, ?4, ?5)",
            (
                event.id.to_string(),
                event.timestamp.to_rfc3339(),
                &event.source,
                event_type_str,
                data,
            ),
        )?;
        Ok(())
    }

    /// Upsert an activity tree node (insert or replace).
    pub fn upsert_activity_node(&self, node: &crate::tree::ActivityNode) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO activity_tree (id, level, parent_id, summary, start_time, end_time) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            (
                &node.id,
                node.level.to_string(),
                node.parent_id.as_deref(),
                &node.summary,
                node.start_time.to_rfc3339(),
                node.end_time.to_rfc3339(),
            ),
        )?;
        Ok(())
    }

    /// Query the activity tree for a specific day.
    pub fn query_tree_for_day(&self, date_str: &str) -> Result<Vec<crate::tree::ActivityNode>> {
        use std::collections::HashMap;

        let mut stmt = self.conn.prepare(
            "SELECT id, level, parent_id, summary, start_time, end_time FROM activity_tree WHERE id LIKE ?1 OR parent_id LIKE ?1 ORDER BY start_time",
        )?;
        let pattern = format!("%{date_str}%");
        let rows = stmt.query_map([&pattern], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
            ))
        })?;

        let mut nodes = Vec::new();
        for row in rows {
            let (id, level_str, parent_id, summary, start_str, end_str) = row?;
            let level = match level_str.as_str() {
                "Day" => crate::tree::TreeLevel::Day,
                "Session" => crate::tree::TreeLevel::Session,
                "App" => crate::tree::TreeLevel::App,
                "Action" => crate::tree::TreeLevel::Action,
                _ => continue,
            };
            let start_time = chrono::DateTime::parse_from_rfc3339(&start_str)
                .map(|t| t.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now());
            let end_time = chrono::DateTime::parse_from_rfc3339(&end_str)
                .map(|t| t.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now());

            nodes.push(crate::tree::ActivityNode {
                id,
                level,
                parent_id,
                summary,
                start_time,
                end_time,
                event_count: 0,
                children: Vec::new(),
                metadata: HashMap::new(),
            });
        }
        Ok(nodes)
    }

    /// Get all events within a time range (for tree building).
    pub fn events_in_range(
        &self,
        start: &chrono::DateTime<chrono::Utc>,
        end: &chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<CatchMeEvent>> {
        let mut stmt = self.conn.prepare(
            "SELECT data FROM events WHERE timestamp >= ?1 AND timestamp <= ?2 ORDER BY timestamp",
        )?;
        let rows = stmt.query_map([start.to_rfc3339(), end.to_rfc3339()], |row| {
            row.get::<_, String>(0)
        })?;

        let mut events = Vec::new();
        for row in rows {
            let data = row?;
            if let Ok(event) = serde_json::from_str::<CatchMeEvent>(&data) {
                events.push(event);
            }
        }
        Ok(events)
    }
}

