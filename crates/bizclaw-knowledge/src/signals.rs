//! Interaction Signal Logger — capture learning signals from every interaction.
//!
//! Core insight:
//! > "Every AI agent running in production is collecting training data...
//! >  but throwing it away."
//!
//! This module captures structured signals from agent interactions for
//! future learning, without needing an RL training pipeline right now.
//!
//! ## Signal Types
//! ```text
//! User: "Doanh thu tháng này bao nhiêu?"
//! Agent: "Doanh thu tháng 3 là 500 triệu VNĐ"
//! User: "Đúng rồi, cảm ơn" ← positive_signal
//!
//! User: "Về khách hàng ABC"
//! Agent: "Khách hàng ABC ở Hà Nội..."
//! User: "Không, ABC ở HCM" ← negative_signal + directive (correction)
//!
//! Tool: shell("ls /tmp") → OK ← tool_success
//! Tool: shell("rm -rf /") → BLOCKED ← tool_blocked
//! ```
//!
//! ## Design
//! - Lightweight SQLite store (same as KB — no new deps)
//! - Signals are structured, not raw text
//! - Can be exported as JSONL for fine-tuning later
//! - Privacy-first: stays local, never uploaded

use rusqlite::params;
use serde::{Deserialize, Serialize};

/// A structured interaction signal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionSignal {
    /// Unique signal ID.
    pub id: String,
    /// Agent that produced this interaction.
    pub agent_name: String,
    /// Session ID for grouping.
    pub session_id: String,
    /// Signal type.
    pub signal_type: SignalType,
    /// The user's original message.
    pub user_message: String,
    /// The agent's response.
    pub agent_response: String,
    /// User's feedback or next-state signal.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feedback: Option<String>,
    /// Computed reward: +1 (positive), 0 (neutral), -1 (negative).
    pub reward: i8,
    /// Timestamp.
    pub created_at: String,
}

/// Types of interaction signals.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SignalType {
    /// User confirmed the response was helpful.
    Positive,
    /// User corrected or rejected the response.
    Negative,
    /// Tool execution succeeded.
    ToolSuccess,
    /// Tool execution failed or was blocked.
    ToolFailure,
    /// Quality gate approved the response.
    QualityApproved,
    /// Quality gate rejected with feedback.
    QualityRejected,
    /// Neutral — no explicit signal from user.
    Neutral,
}

/// Signal logger backed by SQLite.
pub struct SignalLogger {
    conn: rusqlite::Connection,
}

impl SignalLogger {
    /// Open or create the signal database.
    pub fn open(path: &std::path::Path) -> Result<Self, String> {
        let conn = rusqlite::Connection::open(path).map_err(|e| format!("Open signal DB: {e}"))?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS interaction_signals (
                id TEXT PRIMARY KEY,
                agent_name TEXT NOT NULL,
                session_id TEXT NOT NULL,
                signal_type TEXT NOT NULL,
                user_message TEXT NOT NULL,
                agent_response TEXT NOT NULL,
                feedback TEXT,
                reward INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE INDEX IF NOT EXISTS idx_signals_agent
                ON interaction_signals(agent_name);
            CREATE INDEX IF NOT EXISTS idx_signals_type
                ON interaction_signals(signal_type);
            CREATE INDEX IF NOT EXISTS idx_signals_reward
                ON interaction_signals(reward);
            CREATE INDEX IF NOT EXISTS idx_signals_created
                ON interaction_signals(created_at);",
        )
        .map_err(|e| format!("Create signal schema: {e}"))?;

        tracing::debug!("📊 Interaction signal logger ready");
        Ok(Self { conn })
    }

    /// Log a new interaction signal.
    pub fn log(&self, signal: &InteractionSignal) -> Result<(), String> {
        let signal_type_str = serde_json::to_string(&signal.signal_type)
            .unwrap_or_else(|_| "\"neutral\"".into())
            .trim_matches('"')
            .to_string();

        self.conn
            .execute(
                "INSERT INTO interaction_signals
                 (id, agent_name, session_id, signal_type, user_message, agent_response, feedback, reward, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    signal.id,
                    signal.agent_name,
                    signal.session_id,
                    signal_type_str,
                    signal.user_message,
                    signal.agent_response,
                    signal.feedback,
                    signal.reward as i32,
                    signal.created_at,
                ],
            )
            .map_err(|e| format!("Log signal: {e}"))?;
        Ok(())
    }

    /// Detect signal type from user's next message (heuristic-based).
    ///
    /// This is the "next-state signal" concept:
    /// The user's next message after an agent response is a free feedback signal.
    pub fn detect_signal(user_next_message: &str) -> (SignalType, i8) {
        let msg = user_next_message.to_lowercase();

        // Positive signals
        if msg.contains("cảm ơn")
            || msg.contains("thank")
            || msg.contains("đúng rồi")
            || msg.contains("correct")
            || msg.contains("tốt lắm")
            || msg.contains("great")
            || msg.contains("ok")
            || msg.contains("perfect")
            || msg.contains("👍")
            || msg.contains("✅")
            || msg.contains("hay")
            || msg.contains("good")
        {
            return (SignalType::Positive, 1);
        }

        // Negative signals
        if msg.contains("không phải")
            || msg.contains("sai rồi")
            || msg.contains("wrong")
            || msg.contains("incorrect")
            || msg.contains("không đúng")
            || msg.contains("nhầm")
            || msg.contains("lại đi")
            || msg.contains("try again")
            || msg.contains("👎")
            || msg.contains("❌")
            || msg.contains("làm lại")
            || msg.contains("sửa lại")
        {
            return (SignalType::Negative, -1);
        }

        // Directive (correction with content)
        if msg.starts_with("không,")
            || msg.starts_with("sai,")
            || msg.starts_with("actually")
            || msg.starts_with("no,")
        {
            return (SignalType::Negative, -1);
        }

        (SignalType::Neutral, 0)
    }

    /// Get signal statistics for an agent.
    pub fn stats(&self, agent_name: Option<&str>) -> SignalStats {
        let (where_clause, param): (&str, Option<&str>) = match agent_name {
            Some(name) => ("WHERE agent_name = ?1", Some(name)),
            None => ("", None),
        };

        let total: i64 = if let Some(name) = param {
            self.conn
                .query_row(
                    &format!("SELECT COUNT(*) FROM interaction_signals {}", where_clause),
                    params![name],
                    |row| row.get(0),
                )
                .unwrap_or(0)
        } else {
            self.conn
                .query_row("SELECT COUNT(*) FROM interaction_signals", [], |row| {
                    row.get(0)
                })
                .unwrap_or(0)
        };

        let positive: i64 = if let Some(name) = param {
            self.conn
                .query_row(
                    &format!(
                        "SELECT COUNT(*) FROM interaction_signals {} AND reward > 0",
                        if where_clause.is_empty() {
                            "WHERE 1=1"
                        } else {
                            where_clause
                        }
                    ),
                    params![name],
                    |row| row.get(0),
                )
                .unwrap_or(0)
        } else {
            self.conn
                .query_row(
                    "SELECT COUNT(*) FROM interaction_signals WHERE reward > 0",
                    [],
                    |row| row.get(0),
                )
                .unwrap_or(0)
        };

        let negative: i64 = if let Some(name) = param {
            self.conn
                .query_row(
                    &format!(
                        "SELECT COUNT(*) FROM interaction_signals {} AND reward < 0",
                        if where_clause.is_empty() {
                            "WHERE 1=1"
                        } else {
                            where_clause
                        }
                    ),
                    params![name],
                    |row| row.get(0),
                )
                .unwrap_or(0)
        } else {
            self.conn
                .query_row(
                    "SELECT COUNT(*) FROM interaction_signals WHERE reward < 0",
                    [],
                    |row| row.get(0),
                )
                .unwrap_or(0)
        };

        SignalStats {
            total: total as usize,
            positive: positive as usize,
            negative: negative as usize,
            neutral: (total - positive - negative) as usize,
            satisfaction_rate: if total > 0 {
                positive as f64 / total as f64
            } else {
                0.0
            },
        }
    }

    /// Export signals as JSONL (one JSON object per line).
    /// Useful for future fine-tuning.
    pub fn export_jsonl(&self, limit: usize) -> Result<String, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, agent_name, session_id, signal_type, user_message,
                        agent_response, feedback, reward, created_at
                 FROM interaction_signals
                 ORDER BY created_at DESC
                 LIMIT ?1",
            )
            .map_err(|e| format!("Prepare export: {e}"))?;

        let rows = stmt
            .query_map(params![limit as i64], |row| {
                let signal = InteractionSignal {
                    id: row.get(0)?,
                    agent_name: row.get(1)?,
                    session_id: row.get(2)?,
                    signal_type: {
                        let s: String = row.get(3)?;
                        serde_json::from_str(&format!("\"{}\"", s)).unwrap_or(SignalType::Neutral)
                    },
                    user_message: row.get(4)?,
                    agent_response: row.get(5)?,
                    feedback: row.get(6)?,
                    reward: row.get::<_, i32>(7).unwrap_or(0) as i8,
                    created_at: row.get(8)?,
                };
                Ok(signal)
            })
            .map_err(|e| format!("Query export: {e}"))?;

        let mut jsonl = String::new();
        for row in rows {
            if let Ok(signal) = row
                && let Ok(json) = serde_json::to_string(&signal)
            {
                jsonl.push_str(&json);
                jsonl.push('\n');
            }
        }
        Ok(jsonl)
    }

    /// Cleanup old signals (keep last N days).
    pub fn cleanup(&self, keep_days: u32) -> usize {
        let result = self.conn.execute(
            "DELETE FROM interaction_signals WHERE created_at < datetime('now', ?1)",
            params![format!("-{} days", keep_days)],
        );
        match result {
            Ok(count) => {
                if count > 0 {
                    tracing::info!("🧹 Cleaned {} old interaction signal(s)", count);
                }
                count
            }
            Err(_) => 0,
        }
    }
}

/// Aggregated signal statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalStats {
    pub total: usize,
    pub positive: usize,
    pub negative: usize,
    pub neutral: usize,
    /// Positive / Total ratio.
    pub satisfaction_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_logger() -> SignalLogger {
        SignalLogger::open(std::path::Path::new(":memory:")).unwrap()
    }

    #[test]
    fn test_log_and_stats() {
        let logger = test_logger();

        logger
            .log(&InteractionSignal {
                id: "s1".into(),
                agent_name: "sales-bot".into(),
                session_id: "sess1".into(),
                signal_type: SignalType::Positive,
                user_message: "Doanh thu tháng này?".into(),
                agent_response: "500 triệu VNĐ".into(),
                feedback: Some("Đúng rồi, cảm ơn".into()),
                reward: 1,
                created_at: "2025-01-01 10:00:00".into(),
            })
            .unwrap();

        logger
            .log(&InteractionSignal {
                id: "s2".into(),
                agent_name: "sales-bot".into(),
                session_id: "sess1".into(),
                signal_type: SignalType::Negative,
                user_message: "Khách hàng ABC ở đâu?".into(),
                agent_response: "Hà Nội".into(),
                feedback: Some("Sai, ABC ở HCM".into()),
                reward: -1,
                created_at: "2025-01-01 10:05:00".into(),
            })
            .unwrap();

        let stats = logger.stats(Some("sales-bot"));
        assert_eq!(stats.total, 2);
        assert_eq!(stats.positive, 1);
        assert_eq!(stats.negative, 1);
        assert!((stats.satisfaction_rate - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_detect_signal_positive() {
        let (sig, reward) = SignalLogger::detect_signal("Đúng rồi, cảm ơn bạn!");
        assert_eq!(sig, SignalType::Positive);
        assert_eq!(reward, 1);
    }

    #[test]
    fn test_detect_signal_negative() {
        let (sig, reward) = SignalLogger::detect_signal("Không phải, tôi hỏi về ABC");
        assert_eq!(sig, SignalType::Negative);
        assert_eq!(reward, -1);
    }

    #[test]
    fn test_detect_signal_neutral() {
        let (sig, reward) = SignalLogger::detect_signal("Cho tôi hỏi thêm về pricing");
        assert_eq!(sig, SignalType::Neutral);
        assert_eq!(reward, 0);
    }

    #[test]
    fn test_export_jsonl() {
        let logger = test_logger();
        logger
            .log(&InteractionSignal {
                id: "s1".into(),
                agent_name: "bot".into(),
                session_id: "sess".into(),
                signal_type: SignalType::Positive,
                user_message: "test".into(),
                agent_response: "response".into(),
                feedback: None,
                reward: 1,
                created_at: "2025-01-01".into(),
            })
            .unwrap();

        let jsonl = logger.export_jsonl(10).unwrap();
        assert!(jsonl.contains("\"signal_type\":\"positive\""));
        assert!(jsonl.ends_with('\n'));
    }

    #[test]
    fn test_stats_empty() {
        let logger = test_logger();
        let stats = logger.stats(None);
        assert_eq!(stats.total, 0);
        assert_eq!(stats.satisfaction_rate, 0.0);
    }
}
