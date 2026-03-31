//! Database Connection Manager — Config-driven multi-database connector
//!
//! Loads connection profiles from `data/db-connections.json`.
//! Credentials are resolved via `vault://` URIs for AES-256 encrypted storage.
//!
//! # Config format (`data/db-connections.json`):
//! ```json
//! {
//!   "connections": [
//!     {
//!       "id": "prod_mysql",
//!       "db_type": "mysql",
//!       "connection_string": "vault://db_prod_mysql_uri",
//!       "read_only": true,
//!       "max_rows": 500,
//!       "timeout_secs": 15,
//!       "description": "Production MySQL — read-only for AI queries",
//!       "sensitive_columns": ["password", "card_number", "ssn"]
//!     }
//!   ]
//! }
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// A single database connection profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbConnectionProfile {
    /// Unique identifier (e.g., "prod_mysql", "analytics_pg")
    pub id: String,

    /// Database type: "mysql", "postgres", "sqlite"
    pub db_type: String,

    /// Connection string or vault:// URI.
    /// Examples:
    ///   - "postgres://user:pass@host:5432/dbname"
    ///   - "mysql://user:pass@host:3306/dbname"
    ///   - "sqlite:///path/to/db.sqlite"
    ///   - "vault://db_prod_uri"   (resolved at runtime)
    pub connection_string: String,

    /// If true, only SELECT/SHOW/DESCRIBE queries are allowed (enforced by db_safety).
    #[serde(default = "default_true")]
    pub read_only: bool,

    /// Max rows returned per query (default: 100, hard cap: 1000).
    #[serde(default = "default_max_rows")]
    pub max_rows: u32,

    /// Query timeout in seconds (default: 15).
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,

    /// Human-readable description.
    #[serde(default)]
    pub description: String,

    /// Column names that should be redacted in output (PII protection).
    #[serde(default)]
    pub sensitive_columns: Vec<String>,

    /// Optional: restrict which tables the agent can query.
    #[serde(default)]
    pub allowed_tables: Vec<String>,

    /// Whether this connection is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}
fn default_max_rows() -> u32 {
    100
}
fn default_timeout() -> u64 {
    15
}

/// Root config structure for `db-connections.json`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DbConnectionConfig {
    pub connections: Vec<DbConnectionProfile>,
}

/// Connection Manager — loads, validates, and provides DB connections.
pub struct DbConnectionManager {
    profiles: HashMap<String, DbConnectionProfile>,
    config_path: PathBuf,
}

impl DbConnectionManager {
    /// Load connections from the default path: `data/db-connections.json`.
    pub fn load_default() -> Self {
        Self::load_from(Path::new("data/db-connections.json"))
    }

    /// Load connections from a specific path.
    pub fn load_from(path: &Path) -> Self {
        let profiles = match std::fs::read_to_string(path) {
            Ok(content) => match serde_json::from_str::<DbConnectionConfig>(&content) {
                Ok(config) => {
                    let mut map = HashMap::new();
                    for conn in config.connections {
                        if conn.enabled {
                            map.insert(conn.id.clone(), conn);
                        }
                    }
                    tracing::info!(
                        "🗄️ Loaded {} DB connection(s) from {}",
                        map.len(),
                        path.display()
                    );
                    map
                }
                Err(e) => {
                    tracing::warn!("⚠️ Failed to parse DB connections config: {e}");
                    HashMap::new()
                }
            },
            Err(_) => {
                tracing::debug!("No DB connections config at {}", path.display());
                HashMap::new()
            }
        };

        Self {
            profiles,
            config_path: path.to_path_buf(),
        }
    }

    /// Get a connection profile by ID.
    pub fn get(&self, id: &str) -> Option<&DbConnectionProfile> {
        self.profiles.get(id)
    }

    /// List all connection IDs with types and descriptions.
    pub fn list(&self) -> Vec<ConnectionSummary> {
        self.profiles
            .values()
            .map(|p| ConnectionSummary {
                id: p.id.clone(),
                db_type: p.db_type.clone(),
                description: p.description.clone(),
                read_only: p.read_only,
                max_rows: p.max_rows,
            })
            .collect()
    }

    /// Number of registered connections.
    pub fn count(&self) -> usize {
        self.profiles.len()
    }

    /// Config file path.
    pub fn config_path(&self) -> &Path {
        &self.config_path
    }

    /// Resolve a connection_string, using vault if needed.
    /// This requires access to a Vault instance.
    pub fn resolve_connection_string(
        connection_string: &str,
        vault: &bizclaw_security::vault::Vault,
    ) -> String {
        vault.resolve_or_passthrough(connection_string)
    }

    /// Redact sensitive columns from query results.
    #[allow(clippy::ptr_arg)]
    pub fn redact_sensitive(results: &mut Vec<serde_json::Value>, sensitive_columns: &[String]) {
        if sensitive_columns.is_empty() {
            return;
        }
        for row in results.iter_mut() {
            if let Some(obj) = row.as_object_mut() {
                for col in sensitive_columns {
                    if obj.contains_key(col) {
                        obj.insert(
                            col.clone(),
                            serde_json::Value::String("****REDACTED****".into()),
                        );
                    }
                    // Also check case-insensitive
                    let keys: Vec<String> = obj.keys().cloned().collect();
                    for key in keys {
                        if key.to_lowercase() == col.to_lowercase() && key != *col {
                            obj.insert(key, serde_json::Value::String("****REDACTED****".into()));
                        }
                    }
                }
            }
        }
    }

    /// Validate that a query only accesses allowed tables.
    pub fn check_allowed_tables(query: &str, allowed_tables: &[String]) -> Result<(), String> {
        if allowed_tables.is_empty() {
            return Ok(()); // No restriction
        }

        let upper = query.to_uppercase();
        // Simple heuristic: extract table names after FROM and JOIN
        let table_keywords = ["FROM", "JOIN"];
        for kw in &table_keywords {
            for part in upper.split(kw) {
                // The first word after FROM/JOIN is the table name
                if let Some(table_name) = part.split_whitespace().next() {
                    let clean = table_name
                        .trim_matches(|c: char| c == '`' || c == '"' || c == '\'' || c == '(')
                        .to_lowercase();
                    if !clean.is_empty()
                        && clean != "select"
                        && clean != "("
                        && !allowed_tables.iter().any(|t| t.to_lowercase() == clean)
                    {
                        // Skip if it looks like a subquery or keyword
                        let keywords = [
                            "where", "on", "as", "and", "or", "left", "right", "inner", "outer",
                            "cross", "natural", "group", "order", "limit", "having", "union",
                            "set",
                        ];
                        if !keywords.contains(&clean.as_str()) {
                            return Err(format!(
                                "Table '{}' is not in the allowed list: {:?}",
                                clean, allowed_tables
                            ));
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

/// Summary of a connection (safe to expose to agent/UI).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionSummary {
    pub id: String,
    pub db_type: String,
    pub description: String,
    pub read_only: bool,
    pub max_rows: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DbConnectionConfig::default();
        assert!(config.connections.is_empty());
    }

    #[test]
    fn test_parse_config() {
        let json = r#"{
            "connections": [
                {
                    "id": "test_pg",
                    "db_type": "postgres",
                    "connection_string": "postgres://user:pass@localhost/testdb",
                    "read_only": true,
                    "max_rows": 200,
                    "timeout_secs": 10,
                    "description": "Test PostgreSQL",
                    "sensitive_columns": ["password", "ssn"],
                    "allowed_tables": ["users", "orders"],
                    "enabled": true
                },
                {
                    "id": "disabled_db",
                    "db_type": "mysql",
                    "connection_string": "mysql://root@localhost/test",
                    "enabled": false
                }
            ]
        }"#;

        let config: DbConnectionConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.connections.len(), 2);
        assert_eq!(config.connections[0].id, "test_pg");
        assert_eq!(config.connections[0].max_rows, 200);
        assert_eq!(config.connections[0].sensitive_columns.len(), 2);
        assert!(!config.connections[1].enabled);
    }

    #[test]
    fn test_manager_from_json() {
        let json = r#"{
            "connections": [
                {
                    "id": "local_sqlite",
                    "db_type": "sqlite",
                    "connection_string": "sqlite:///tmp/test.db",
                    "description": "Local test DB"
                }
            ]
        }"#;

        // Write to temp file
        let path = std::path::PathBuf::from("/tmp/test_db_connections.json");
        std::fs::write(&path, json).unwrap();

        let mgr = DbConnectionManager::load_from(&path);
        assert_eq!(mgr.count(), 1);
        assert!(mgr.get("local_sqlite").is_some());
        assert_eq!(mgr.get("local_sqlite").unwrap().db_type, "sqlite");
        assert!(mgr.get("nonexistent").is_none());

        let list = mgr.list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "local_sqlite");

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_redact_sensitive() {
        let mut results = vec![
            serde_json::json!({"name": "Alice", "password": "secret123", "email": "a@b.com"}),
            serde_json::json!({"name": "Bob", "password": "hunter2", "ssn": "123-45-6789"}),
        ];

        DbConnectionManager::redact_sensitive(&mut results, &["password".into(), "ssn".into()]);

        assert_eq!(results[0]["password"], "****REDACTED****");
        assert_eq!(results[0]["name"], "Alice");
        assert_eq!(results[1]["password"], "****REDACTED****");
        assert_eq!(results[1]["ssn"], "****REDACTED****");
        assert_eq!(results[1]["name"], "Bob");
    }

    #[test]
    fn test_allowed_tables_pass() {
        let result = DbConnectionManager::check_allowed_tables(
            "SELECT * FROM users WHERE id = 1",
            &["users".into(), "orders".into()],
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_allowed_tables_block() {
        let result = DbConnectionManager::check_allowed_tables(
            "SELECT * FROM payments WHERE amount > 100",
            &["users".into(), "orders".into()],
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("payments"));
    }

    #[test]
    fn test_allowed_tables_no_restriction() {
        let result = DbConnectionManager::check_allowed_tables("SELECT * FROM anything_goes", &[]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_load_nonexistent_file() {
        let mgr = DbConnectionManager::load_from(Path::new("/tmp/nonexistent_db_cfg.json"));
        assert_eq!(mgr.count(), 0);
    }
}
