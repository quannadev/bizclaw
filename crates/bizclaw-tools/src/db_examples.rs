//! SQL Example Store — Learn from past successful NL→SQL pairs
//!
//! Smart example matching for Text2SQL pipeline.
//! Uses SQLite FTS5 to store & retrieve similar question→SQL pairs.
//!
//! Flow:
//! 1. User asks NL question → AI generates SQL → executes → user confirms correct
//! 2. Save {question, sql, connection_id, tables_used} → FTS5
//! 3. Next time similar question → find matching examples → inject as few-shot
//! 4. Accuracy improves over time (self-learning RAG)

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// A single NL→SQL example pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqlExample {
    pub id: i64,
    pub question: String,
    pub normalized_question: String,
    pub sql: String,
    pub connection_id: String,
    pub tables_used: Vec<String>,
    pub created_at: String,
    pub verified: bool,
}

/// Store for SQL examples using SQLite + FTS5.
pub struct SqlExampleStore {
    db_path: PathBuf,
}

impl SqlExampleStore {
    /// Create or open the example store.
    pub fn new(data_dir: &Path) -> Self {
        let db_path = data_dir.join("db-examples.sqlite");
        let store = Self { db_path };
        if let Err(e) = store.init_db() {
            tracing::warn!("[db-examples] Failed to init: {e}");
        }
        store
    }

    /// Default path: data/db-examples.sqlite
    pub fn default() -> Self {
        Self::new(Path::new("data"))
    }

    fn init_db(&self) -> Result<(), String> {
        let conn =
            rusqlite::Connection::open(&self.db_path).map_err(|e| format!("Open DB: {e}"))?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS sql_examples (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                question TEXT NOT NULL,
                normalized_question TEXT NOT NULL,
                sql_code TEXT NOT NULL,
                connection_id TEXT NOT NULL,
                tables_used TEXT DEFAULT '',
                created_at TEXT DEFAULT (datetime('now')),
                verified INTEGER DEFAULT 0
            );
            CREATE VIRTUAL TABLE IF NOT EXISTS sql_examples_fts USING fts5(
                question, normalized_question, sql_code,
                content='sql_examples',
                content_rowid='id'
            );
            CREATE TRIGGER IF NOT EXISTS sql_examples_ai AFTER INSERT ON sql_examples BEGIN
                INSERT INTO sql_examples_fts(rowid, question, normalized_question, sql_code)
                VALUES (new.id, new.question, new.normalized_question, new.sql_code);
            END;
            CREATE TRIGGER IF NOT EXISTS sql_examples_ad AFTER DELETE ON sql_examples BEGIN
                INSERT INTO sql_examples_fts(sql_examples_fts, rowid, question, normalized_question, sql_code)
                VALUES ('delete', old.id, old.question, old.normalized_question, old.sql_code);
            END;"
        ).map_err(|e| format!("Init tables: {e}"))?;

        Ok(())
    }

    /// Save a new NL→SQL example.
    pub fn save(
        &self,
        question: &str,
        normalized_question: &str,
        sql: &str,
        connection_id: &str,
        tables_used: &[String],
    ) -> Result<i64, String> {
        let conn =
            rusqlite::Connection::open(&self.db_path).map_err(|e| format!("Open DB: {e}"))?;

        let tables_str = tables_used.join(",");
        conn.execute(
            "INSERT INTO sql_examples (question, normalized_question, sql_code, connection_id, tables_used, verified)
             VALUES (?1, ?2, ?3, ?4, ?5, 1)",
            rusqlite::params![question, normalized_question, sql, connection_id, tables_str],
        ).map_err(|e| format!("Insert: {e}"))?;

        Ok(conn.last_insert_rowid())
    }

    /// Find similar examples using FTS5 search.
    /// Returns up to `limit` examples sorted by relevance.
    pub fn find_similar(
        &self,
        question: &str,
        connection_id: &str,
        limit: usize,
    ) -> Vec<SqlExample> {
        let conn = match rusqlite::Connection::open(&self.db_path) {
            Ok(c) => c,
            Err(_) => return vec![],
        };

        // Sanitize for FTS5: remove special chars
        let sanitized = question
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace())
            .collect::<String>();

        if sanitized.trim().is_empty() {
            return vec![];
        }

        // FTS5 search with ranking
        let query = "SELECT e.id, e.question, e.normalized_question, e.sql_code,
                    e.connection_id, e.tables_used, e.created_at, e.verified
             FROM sql_examples e
             JOIN sql_examples_fts f ON e.id = f.rowid
             WHERE sql_examples_fts MATCH ?1
               AND e.connection_id = ?2
               AND e.verified = 1
             ORDER BY rank
             LIMIT ?3"
            .to_string();

        let mut stmt = match conn.prepare(&query) {
            Ok(s) => s,
            Err(_) => return vec![],
        };

        let results = stmt.query_map(
            rusqlite::params![sanitized, connection_id, limit as i64],
            |row| {
                let tables_str: String = row.get(5).unwrap_or_default();
                Ok(SqlExample {
                    id: row.get(0)?,
                    question: row.get(1)?,
                    normalized_question: row.get(2)?,
                    sql: row.get(3)?,
                    connection_id: row.get(4)?,
                    tables_used: tables_str
                        .split(',')
                        .filter(|s| !s.is_empty())
                        .map(|s| s.to_string())
                        .collect(),
                    created_at: row.get(6)?,
                    verified: row.get::<_, i32>(7).unwrap_or(0) == 1,
                })
            },
        );

        match results {
            Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
            Err(_) => vec![],
        }
    }

    /// Get total count of examples.
    pub fn count(&self) -> usize {
        let conn = match rusqlite::Connection::open(&self.db_path) {
            Ok(c) => c,
            Err(_) => return 0,
        };
        conn.query_row("SELECT COUNT(*) FROM sql_examples", [], |row| {
            row.get::<_, i64>(0)
        })
        .unwrap_or(0) as usize
    }

    /// List recent examples for a connection.
    pub fn list_recent(&self, connection_id: &str, limit: usize) -> Vec<SqlExample> {
        let conn = match rusqlite::Connection::open(&self.db_path) {
            Ok(c) => c,
            Err(_) => return vec![],
        };

        let mut stmt = match conn.prepare(
            "SELECT id, question, normalized_question, sql_code, connection_id, tables_used, created_at, verified
             FROM sql_examples WHERE connection_id = ?1 ORDER BY id DESC LIMIT ?2"
        ) {
            Ok(s) => s,
            Err(_) => return vec![],
        };

        let results = stmt.query_map(rusqlite::params![connection_id, limit as i64], |row| {
            let tables_str: String = row.get(5).unwrap_or_default();
            Ok(SqlExample {
                id: row.get(0)?,
                question: row.get(1)?,
                normalized_question: row.get(2)?,
                sql: row.get(3)?,
                connection_id: row.get(4)?,
                tables_used: tables_str
                    .split(',')
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .collect(),
                created_at: row.get(6)?,
                verified: row.get::<_, i32>(7).unwrap_or(0) == 1,
            })
        });

        match results {
            Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
            Err(_) => vec![],
        }
    }

    /// Delete an example by ID.
    pub fn delete(&self, id: i64) -> Result<(), String> {
        let conn =
            rusqlite::Connection::open(&self.db_path).map_err(|e| format!("Open DB: {e}"))?;
        conn.execute("DELETE FROM sql_examples WHERE id = ?1", [id])
            .map_err(|e| format!("Delete: {e}"))?;
        Ok(())
    }
}

/// Business rules for a database connection.
/// Users can define rules that the LLM must follow when generating SQL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusinessRule {
    pub id: String,
    pub connection_id: String,
    pub rule: String,
    pub created_at: Option<String>,
}

/// Store for business rules (JSON file based).
pub struct BusinessRuleStore {
    path: PathBuf,
}

impl BusinessRuleStore {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            path: data_dir.join("db-business-rules.json"),
        }
    }

    pub fn default() -> Self {
        Self::new(Path::new("data"))
    }

    /// Load all rules.
    pub fn load_all(&self) -> Vec<BusinessRule> {
        match std::fs::read_to_string(&self.path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => vec![],
        }
    }

    /// Get rules for a specific connection.
    pub fn get_rules(&self, connection_id: &str) -> Vec<BusinessRule> {
        self.load_all()
            .into_iter()
            .filter(|r| r.connection_id == connection_id || r.connection_id == "*")
            .collect()
    }

    /// Add a rule.
    pub fn add_rule(&self, connection_id: &str, rule: &str) -> Result<String, String> {
        let mut rules = self.load_all();
        let id = format!("rule_{}", rules.len() + 1);
        rules.push(BusinessRule {
            id: id.clone(),
            connection_id: connection_id.to_string(),
            rule: rule.to_string(),
            created_at: Some(chrono::Utc::now().to_rfc3339()),
        });
        let json = serde_json::to_string_pretty(&rules).map_err(|e| format!("Serialize: {e}"))?;
        std::fs::write(&self.path, json).map_err(|e| format!("Write: {e}"))?;
        Ok(id)
    }

    /// Delete a rule by ID.
    pub fn delete_rule(&self, rule_id: &str) -> Result<(), String> {
        let mut rules = self.load_all();
        rules.retain(|r| r.id != rule_id);
        let json = serde_json::to_string_pretty(&rules).map_err(|e| format!("Serialize: {e}"))?;
        std::fs::write(&self.path, json).map_err(|e| format!("Write: {e}"))?;
        Ok(())
    }

    /// Format rules for LLM prompt injection.
    pub fn format_for_prompt(&self, connection_id: &str) -> String {
        let rules = self.get_rules(connection_id);
        if rules.is_empty() {
            return String::new();
        }
        let items: Vec<String> = rules
            .iter()
            .enumerate()
            .map(|(i, r)| format!("{}. {}", i + 1, r.rule))
            .collect();
        format!("MANDATORY Business Rules:\n{}", items.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_example_store_crud() {
        let dir = PathBuf::from("/tmp/bizclaw_test_examples");
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::remove_file(dir.join("db-examples.sqlite"));

        let store = SqlExampleStore::new(&dir);

        // Save
        let id = store
            .save(
                "doanh thu tháng này",
                "revenue current month",
                "SELECT SUM(total) FROM orders WHERE created_at >= date('now', 'start of month')",
                "prod_pg",
                &["orders".to_string()],
            )
            .unwrap();
        assert!(id > 0);

        // Count
        assert_eq!(store.count(), 1);

        // List recent
        let recent = store.list_recent("prod_pg", 10);
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].question, "doanh thu tháng này");

        // Find similar
        let found = store.find_similar("doanh thu", "prod_pg", 5);
        assert!(!found.is_empty());

        // Delete
        store.delete(id).unwrap();
        assert_eq!(store.count(), 0);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_business_rules() {
        let dir = PathBuf::from("/tmp/bizclaw_test_rules");
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::remove_file(dir.join("db-business-rules.json"));

        let store = BusinessRuleStore::new(&dir);

        // Add rules
        store
            .add_rule(
                "prod_pg",
                "Revenue = SUM(order_items.quantity * order_items.price)",
            )
            .unwrap();
        store
            .add_rule(
                "*",
                "Always exclude deleted records: WHERE deleted_at IS NULL",
            )
            .unwrap();

        // Get for connection
        let rules = store.get_rules("prod_pg");
        assert_eq!(rules.len(), 2); // 1 specific + 1 global

        // Format for prompt
        let prompt = store.format_for_prompt("prod_pg");
        assert!(prompt.contains("Revenue"));
        assert!(prompt.contains("deleted_at"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
