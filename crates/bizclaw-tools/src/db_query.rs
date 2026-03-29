//! Database Query tool — Read-only SQL executor with Connection Manager + Vault integration
//!
//! Supports two modes:
//! 1. `connection_id` — uses pre-configured profile from `data/db-connections.json` (recommended)
//! 2. `connection_string` — direct URI (for ad-hoc queries, less secure)
//!
//! Safety layers:
//! - SQL statement allowlist (SELECT/SHOW/DESCRIBE/EXPLAIN/WITH only)
//! - Row limit (max 1000)
//! - Query timeout
//! - PII column redaction
//! - Table allowlist
//! - Vault-encrypted credentials

use crate::db_connection::{DbConnectionManager, DbConnectionProfile};
use crate::db_safety::DbSafety;
use async_trait::async_trait;
use bizclaw_core::error::Result;
use bizclaw_core::traits::Tool;
use bizclaw_core::types::{ToolDefinition, ToolResult};
use sqlx::any::AnyPoolOptions;
use sqlx::{Column, Row, ValueRef};

pub struct DbQueryTool {
    connection_manager: DbConnectionManager,
}

impl DbQueryTool {
    pub fn new() -> Self {
        sqlx::any::install_default_drivers();
        Self {
            connection_manager: DbConnectionManager::load_default(),
        }
    }

    /// Create with a custom config path (for testing).
    pub fn with_config(config_path: &std::path::Path) -> Self {
        sqlx::any::install_default_drivers();
        Self {
            connection_manager: DbConnectionManager::load_from(config_path),
        }
    }
}

impl Default for DbQueryTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for DbQueryTool {
    fn name(&self) -> &str {
        "db_query"
    }

    fn definition(&self) -> ToolDefinition {
        // Build available connections list for the description
        let connections = self.connection_manager.list();
        let conn_list = if connections.is_empty() {
            "No database connections configured. Add them in data/db-connections.json".to_string()
        } else {
            connections
                .iter()
                .map(|c| format!("  - '{}' ({}) — {}", c.id, c.db_type, c.description))
                .collect::<Vec<_>>()
                .join("\n")
        };

        ToolDefinition {
            name: "db_query".into(),
            description: format!(
                "Execute read-only SQL queries on configured databases. ONLY SELECT, SHOW, DESCRIBE, EXPLAIN queries are allowed. \
                DELETE, DROP, UPDATE, INSERT are strictly forbidden.\n\n\
                Available connections:\n{}\n\n\
                Use 'connection_id' to select a pre-configured database, or 'connection_string' for ad-hoc connections.",
                conn_list
            ),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "connection_id": {
                        "type": "string",
                        "description": "ID of a pre-configured database connection (recommended). See available connections in the tool description."
                    },
                    "connection_string": {
                        "type": "string",
                        "description": "Direct database URI (fallback). Format: postgres://user:pass@host/db or mysql://user:pass@host/db or sqlite:///path/to/db"
                    },
                    "query": {
                        "type": "string",
                        "description": "SQL query to execute. ONLY SELECT/SHOW/DESCRIBE/EXPLAIN allowed."
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Max rows to return (default: 100, max: 1000)"
                    }
                },
                "required": ["query"]
            }),
        }
    }

    async fn execute(&self, arguments: &str) -> Result<ToolResult> {
        let args: serde_json::Value = serde_json::from_str(arguments).map_err(|e| {
            bizclaw_core::error::BizClawError::Tool(format!("Invalid arguments JSON: {}", e))
        })?;

        let query = args["query"]
            .as_str()
            .ok_or_else(|| bizclaw_core::error::BizClawError::Tool("Missing 'query'".into()))?;

        // ── Safety Check ──
        if let Err(msg) = DbSafety::ensure_safe_query(query) {
            return Ok(ToolResult {
                tool_call_id: String::new(),
                output: format!("🛡️ Safety Block: {}", msg),
                success: false,
            });
        }

        // ── Resolve connection ──
        let (uri, profile) = self.resolve_connection(&args)?;

        // Apply profile limits
        let profile_max_rows = profile.as_ref().map(|p| p.max_rows).unwrap_or(100);
        let user_limit = args["limit"].as_u64().unwrap_or(profile_max_rows as u64);
        let limit = std::cmp::min(user_limit, 1000) as usize; // hard cap

        let timeout_secs = profile.as_ref().map(|p| p.timeout_secs).unwrap_or(15);

        // ── Table allowlist check ──
        if let Some(ref p) = profile
            && !p.allowed_tables.is_empty()
            && let Err(msg) = DbConnectionManager::check_allowed_tables(query, &p.allowed_tables)
        {
            return Ok(ToolResult {
                tool_call_id: String::new(),
                output: format!("🛡️ Table Access Denied: {}", msg),
                success: false,
            });
        }

        // ── Connect ──
        let start = std::time::Instant::now();
        let pool = match AnyPoolOptions::new()
            .max_connections(2)
            .acquire_timeout(std::time::Duration::from_secs(timeout_secs))
            .connect(&uri)
            .await
        {
            Ok(p) => p,
            Err(e) => {
                return Ok(ToolResult {
                    tool_call_id: String::new(),
                    output: format!("❌ Connection failed: {}", e),
                    success: false,
                });
            }
        };

        // ── Execute Query ──
        let rows = match tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            sqlx::query(query).fetch_all(&pool),
        )
        .await
        {
            Ok(Ok(r)) => r,
            Ok(Err(e)) => {
                return Ok(ToolResult {
                    tool_call_id: String::new(),
                    output: format!("❌ Query failed: {}", e),
                    success: false,
                });
            }
            Err(_) => {
                return Ok(ToolResult {
                    tool_call_id: String::new(),
                    output: format!("⏰ Query timed out after {}s", timeout_secs),
                    success: false,
                });
            }
        };

        let total_rows = rows.len();

        // ── Format Results ──
        let mut results = Vec::new();
        for row in rows.into_iter().take(limit) {
            let mut row_map = serde_json::Map::new();
            for col in row.columns() {
                let col_name = col.name();
                let val_str = match row.try_get_raw(col_name) {
                    Ok(raw_val) => {
                        if raw_val.is_null() {
                            "NULL".to_string()
                        } else if let Ok(s) = row.try_get::<&str, _>(col_name) {
                            s.to_string()
                        } else if let Ok(s) = row.try_get::<String, _>(col_name) {
                            s
                        } else if let Ok(i) = row.try_get::<i64, _>(col_name) {
                            i.to_string()
                        } else if let Ok(i) = row.try_get::<i32, _>(col_name) {
                            i.to_string()
                        } else if let Ok(f) = row.try_get::<f64, _>(col_name) {
                            f.to_string()
                        } else if let Ok(b) = row.try_get::<bool, _>(col_name) {
                            b.to_string()
                        } else {
                            "[Binary/Unsupported]".to_string()
                        }
                    }
                    Err(_) => "ERR".to_string(),
                };
                row_map.insert(col_name.to_string(), serde_json::Value::String(val_str));
            }
            results.push(serde_json::Value::Object(row_map));
        }

        // ── Redact sensitive columns ──
        if let Some(ref p) = profile
            && !p.sensitive_columns.is_empty()
        {
            DbConnectionManager::redact_sensitive(&mut results, &p.sensitive_columns);
        }

        let elapsed = start.elapsed();
        let conn_id_str = profile
            .as_ref()
            .map(|p| format!(" [{}]", p.id))
            .unwrap_or_default();

        let truncated_note = if total_rows > limit {
            format!(" (showing {}/{} rows, limit={})", limit, total_rows, limit)
        } else {
            String::new()
        };

        let output = format!(
            "✅ Query returned {} rows in {}ms{}{}:\n{}",
            results.len(),
            elapsed.as_millis(),
            conn_id_str,
            truncated_note,
            serde_json::to_string_pretty(&results).unwrap_or_else(|_| "[]".into())
        );

        Ok(ToolResult {
            tool_call_id: String::new(),
            output,
            success: true,
        })
    }
}

impl DbQueryTool {
    /// Resolve connection URI from either connection_id (preferred) or raw connection_string.
    fn resolve_connection(
        &self,
        args: &serde_json::Value,
    ) -> Result<(String, Option<DbConnectionProfile>)> {
        // Priority 1: connection_id → look up in connection manager
        if let Some(conn_id) = args["connection_id"].as_str() {
            if let Some(profile) = self.connection_manager.get(conn_id) {
                // Resolve vault:// URIs
                let vault = bizclaw_security::vault::Vault::new();
                let uri = DbConnectionManager::resolve_connection_string(
                    &profile.connection_string,
                    &vault,
                );
                return Ok((uri, Some(profile.clone())));
            } else {
                let available = self
                    .connection_manager
                    .list()
                    .iter()
                    .map(|c| c.id.clone())
                    .collect::<Vec<_>>()
                    .join(", ");
                return Err(bizclaw_core::error::BizClawError::Tool(format!(
                    "Unknown connection_id '{}'. Available: [{}]",
                    conn_id, available
                )));
            }
        }

        // Priority 2: raw connection_string
        if let Some(uri) = args["connection_string"].as_str() {
            // Still resolve vault:// if used inline
            let vault = bizclaw_security::vault::Vault::new();
            let resolved = vault.resolve_or_passthrough(uri);
            return Ok((resolved, None));
        }

        // No connection specified
        let available = self
            .connection_manager
            .list()
            .iter()
            .map(|c| format!("'{}' ({})", c.id, c.db_type))
            .collect::<Vec<_>>()
            .join(", ");
        Err(bizclaw_core::error::BizClawError::Tool(format!(
            "Either 'connection_id' or 'connection_string' is required. Available connections: [{}]",
            available
        )))
    }
}
