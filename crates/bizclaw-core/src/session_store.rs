//! JSONL Session Persistence — save/restore complete conversation history.
//!
//! Ported from smallnest/goclaw's JSONL session system.
//! Each conversation turn (user → tool calls → response) is written as one JSON line.
//! This enables crash recovery, session resume, and tool call chain debugging.
//!
//! ## Format
//! ```text
//! {"ts":"2026-03-24T22:00:00Z","turn":1,"role":"user","content":"Hello"}
//! {"ts":"2026-03-24T22:00:01Z","turn":1,"role":"assistant","content":"Hi!","tool_calls":[...]}
//! {"ts":"2026-03-24T22:00:02Z","turn":1,"role":"tool","content":"{...}","tool_call_id":"tc_1"}
//! ```
//!
//! ## Storage
//! Sessions are stored at: `~/.bizclaw/sessions/{session_id}.jsonl`

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{debug, info, warn};

use crate::types::ToolCall;
use crate::types::message::{Message, Role};

/// A single entry in the JSONL session file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEntry {
    /// Timestamp of this entry.
    pub ts: DateTime<Utc>,
    /// Turn number (increments on each user message).
    pub turn: u32,
    /// Message role.
    pub role: Role,
    /// Message content.
    pub content: String,
    /// Tool call ID (for tool responses).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// Tool calls made by the assistant.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    /// Model used for this turn (only for assistant messages).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Token count for this entry.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens: Option<u64>,
}

impl SessionEntry {
    /// Create from a Message + turn number.
    pub fn from_message(msg: &Message, turn: u32) -> Self {
        Self {
            ts: Utc::now(),
            turn,
            role: msg.role.clone(),
            content: msg.content.clone(),
            tool_call_id: msg.tool_call_id.clone(),
            tool_calls: msg.tool_calls.clone(),
            model: None,
            tokens: None,
        }
    }

    /// Create with model info (for assistant messages).
    pub fn from_message_with_model(msg: &Message, turn: u32, model: &str, tokens: u64) -> Self {
        let mut entry = Self::from_message(msg, turn);
        entry.model = Some(model.to_string());
        entry.tokens = Some(tokens);
        entry
    }

    /// Convert back to a Message.
    pub fn to_message(&self) -> Message {
        Message {
            role: self.role.clone(),
            content: self.content.clone(),
            name: None,
            tool_call_id: self.tool_call_id.clone(),
            tool_calls: self.tool_calls.clone(),
        }
    }
}

/// JSONL Session Store — persists conversation to disk.
pub struct SessionStore {
    /// Base directory for session files.
    base_dir: PathBuf,
}

impl SessionStore {
    /// Create a new session store.
    /// Default base dir: `~/.bizclaw/sessions/`
    pub fn new() -> Self {
        let base = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".bizclaw")
            .join("sessions");
        Self { base_dir: base }
    }

    /// Create with a custom base directory.
    pub fn with_dir(dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: dir.into(),
        }
    }

    /// Ensure the sessions directory exists.
    fn ensure_dir(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.base_dir)
    }

    /// Get the JSONL file path for a session.
    fn session_path(&self, session_id: &str) -> PathBuf {
        self.base_dir
            .join(format!("{}.jsonl", sanitize_filename(session_id)))
    }

    /// Append a single entry to the session file.
    pub fn append(&self, session_id: &str, entry: &SessionEntry) -> crate::Result<()> {
        self.ensure_dir()
            .map_err(|e| crate::BizClawError::Config(format!("Create sessions dir: {e}")))?;

        let path = self.session_path(session_id);
        let line = serde_json::to_string(entry)
            .map_err(|e| crate::BizClawError::Config(format!("Serialize session entry: {e}")))?;

        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| {
                crate::BizClawError::Config(format!("Open session file {}: {e}", path.display()))
            })?;

        writeln!(file, "{}", line)
            .map_err(|e| crate::BizClawError::Config(format!("Write session entry: {e}")))?;

        debug!(
            "Session {}: appended turn {} ({})",
            session_id, entry.turn, entry.role
        );
        Ok(())
    }

    /// Append a batch of messages for a single turn.
    pub fn append_turn(
        &self,
        session_id: &str,
        turn: u32,
        messages: &[Message],
    ) -> crate::Result<()> {
        for msg in messages {
            let entry = SessionEntry::from_message(msg, turn);
            self.append(session_id, &entry)?;
        }
        Ok(())
    }

    /// Load all entries from a session.
    pub fn load(&self, session_id: &str) -> crate::Result<Vec<SessionEntry>> {
        let path = self.session_path(session_id);
        if !path.exists() {
            return Ok(Vec::new());
        }

        let content = std::fs::read_to_string(&path).map_err(|e| {
            crate::BizClawError::Config(format!("Read session {}: {e}", path.display()))
        })?;

        let mut entries = Vec::new();
        for (line_num, line) in content.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<SessionEntry>(line) {
                Ok(entry) => entries.push(entry),
                Err(e) => {
                    warn!(
                        "Session {}: skipping corrupt line {} — {e}",
                        session_id,
                        line_num + 1
                    );
                }
            }
        }

        info!("Session {}: loaded {} entries", session_id, entries.len());
        Ok(entries)
    }

    /// Load session and convert to messages (for context injection).
    pub fn load_messages(&self, session_id: &str) -> crate::Result<Vec<Message>> {
        let entries = self.load(session_id)?;
        Ok(entries.iter().map(|e| e.to_message()).collect())
    }

    /// Get the last turn number for a session.
    pub fn last_turn(&self, session_id: &str) -> crate::Result<u32> {
        let entries = self.load(session_id)?;
        Ok(entries.iter().map(|e| e.turn).max().unwrap_or(0))
    }

    /// List all session IDs.
    pub fn list_sessions(&self) -> crate::Result<Vec<SessionInfo>> {
        self.ensure_dir()
            .map_err(|e| crate::BizClawError::Config(format!("Create sessions dir: {e}")))?;

        let mut sessions = Vec::new();
        let entries = std::fs::read_dir(&self.base_dir)
            .map_err(|e| crate::BizClawError::Config(format!("Read sessions dir: {e}")))?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "jsonl") {
                let id = path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let meta = std::fs::metadata(&path).ok();
                sessions.push(SessionInfo {
                    id,
                    size_bytes: meta.as_ref().map(|m| m.len()).unwrap_or(0),
                    modified: meta
                        .and_then(|m| m.modified().ok())
                        .map(DateTime::<Utc>::from),
                });
            }
        }

        sessions.sort_by(|a, b| b.modified.cmp(&a.modified));
        Ok(sessions)
    }

    /// Delete a session.
    pub fn delete(&self, session_id: &str) -> crate::Result<bool> {
        let path = self.session_path(session_id);
        if path.exists() {
            std::fs::remove_file(&path)
                .map_err(|e| crate::BizClawError::Config(format!("Delete session: {e}")))?;
            info!("Session deleted: {}", session_id);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Get total disk usage for all sessions.
    pub fn total_usage_bytes(&self) -> u64 {
        std::fs::read_dir(&self.base_dir)
            .ok()
            .map(|entries| {
                entries
                    .flatten()
                    .filter(|e| e.path().extension().is_some_and(|ext| ext == "jsonl"))
                    .filter_map(|e| e.metadata().ok())
                    .map(|m| m.len())
                    .sum()
            })
            .unwrap_or(0)
    }
}

impl Default for SessionStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Session listing info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub size_bytes: u64,
    pub modified: Option<DateTime<Utc>>,
}

/// Sanitize a session ID for use as a filename.
fn sanitize_filename(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn store_in_tmp() -> (SessionStore, TempDir) {
        let dir = TempDir::new().unwrap();
        let store = SessionStore::with_dir(dir.path());
        (store, dir)
    }

    #[test]
    fn test_append_and_load() {
        let (store, _dir) = store_in_tmp();
        let session = "test-session-1";

        let msg_user = Message::user("Xin chào");
        let msg_asst = Message::assistant("Chào bạn! Tôi có thể giúp gì?");

        store
            .append(session, &SessionEntry::from_message(&msg_user, 1))
            .unwrap();
        store
            .append(session, &SessionEntry::from_message(&msg_asst, 1))
            .unwrap();

        let entries = store.load(session).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].role, Role::User);
        assert_eq!(entries[1].role, Role::Assistant);
        assert_eq!(entries[0].content, "Xin chào");
    }

    #[test]
    fn test_append_turn() {
        let (store, _dir) = store_in_tmp();
        let session = "test-turn-batch";

        let messages = vec![Message::user("What is 2+2?"), Message::assistant("4")];

        store.append_turn(session, 1, &messages).unwrap();
        assert_eq!(store.last_turn(session).unwrap(), 1);
    }

    #[test]
    fn test_load_messages() {
        let (store, _dir) = store_in_tmp();
        let session = "test-messages";

        store
            .append(
                session,
                &SessionEntry::from_message(&Message::user("Hi"), 1),
            )
            .unwrap();
        store
            .append(
                session,
                &SessionEntry::from_message(&Message::assistant("Hello!"), 1),
            )
            .unwrap();

        let messages = store.load_messages(session).unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, Role::User);
    }

    #[test]
    fn test_list_sessions() {
        let (store, _dir) = store_in_tmp();

        store
            .append(
                "sess-a",
                &SessionEntry::from_message(&Message::user("a"), 1),
            )
            .unwrap();
        store
            .append(
                "sess-b",
                &SessionEntry::from_message(&Message::user("b"), 1),
            )
            .unwrap();

        let sessions = store.list_sessions().unwrap();
        assert_eq!(sessions.len(), 2);
    }

    #[test]
    fn test_delete_session() {
        let (store, _dir) = store_in_tmp();
        let session = "to-delete";

        store
            .append(
                session,
                &SessionEntry::from_message(&Message::user("bye"), 1),
            )
            .unwrap();
        assert!(store.delete(session).unwrap());
        assert!(!store.delete(session).unwrap()); // already deleted

        let entries = store.load(session).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_corrupt_line_recovery() {
        let (store, _dir) = store_in_tmp();
        let session = "corrupt-test";

        // Write valid entry
        store
            .append(
                session,
                &SessionEntry::from_message(&Message::user("ok"), 1),
            )
            .unwrap();

        // Manually append corrupt line
        let path = store.session_path(session);
        std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap()
            .write_all(b"THIS IS NOT JSON\n")
            .unwrap();

        use std::io::Write;
        // Write another valid entry
        store
            .append(
                session,
                &SessionEntry::from_message(&Message::user("still ok"), 2),
            )
            .unwrap();

        let entries = store.load(session).unwrap();
        assert_eq!(entries.len(), 2); // skips corrupt line
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("normal-id_123"), "normal-id_123");
        assert_eq!(sanitize_filename("user@host:path"), "user_host_path");
        assert_eq!(sanitize_filename("../../../etc"), "_________etc");
    }

    #[test]
    fn test_with_model_info() {
        let msg = Message::assistant("Hello");
        let entry = SessionEntry::from_message_with_model(&msg, 1, "gpt-4o-mini", 150);
        assert_eq!(entry.model.as_deref(), Some("gpt-4o-mini"));
        assert_eq!(entry.tokens, Some(150));
    }
}
