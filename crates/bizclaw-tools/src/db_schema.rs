//! Database Schema Discovery tool — AI tự khám phá cấu trúc database
//!
//! Đây là bước đầu tiên trong flow RAG-from-DB:
//! 1. `db_schema` → AI biết DB có bảng gì, cột gì, quan hệ gì
//! 2. `db_query` → AI tự viết SQL dựa trên schema đã hiểu
//! 3. AI phân tích data → trả lời câu hỏi kinh doanh
//!
//! Flow cho doanh nghiệp:
//! ```text
//! User: "Doanh thu tháng này so với tháng trước?"
//! Agent:
//!   1. db_schema → biết bảng orders có cột total, created_at
//!   2. db_query → SELECT SUM(total) FROM orders WHERE ...
//!   3. Phân tích → "Doanh thu tháng 3: 150M, tăng 23% so với tháng 2"
//! ```

use crate::db_connection::DbConnectionManager;
use async_trait::async_trait;
use bizclaw_core::error::Result;
use bizclaw_core::traits::Tool;
use bizclaw_core::types::{ToolDefinition, ToolResult};
use sqlx::any::AnyPoolOptions;
use sqlx::{Column, Row, ValueRef};

pub struct DbSchemaTool {
    connection_manager: DbConnectionManager,
}

impl DbSchemaTool {
    pub fn new() -> Self {
        sqlx::any::install_default_drivers();
        Self {
            connection_manager: DbConnectionManager::load_default(),
        }
    }
}

impl Default for DbSchemaTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for DbSchemaTool {
    fn name(&self) -> &str {
        "db_schema"
    }

    fn definition(&self) -> ToolDefinition {
        let connections = self.connection_manager.list();
        let conn_list = if connections.is_empty() {
            "No database connections configured.".to_string()
        } else {
            connections
                .iter()
                .map(|c| format!("  - '{}' ({}) — {}", c.id, c.db_type, c.description))
                .collect::<Vec<_>>()
                .join("\n")
        };

        ToolDefinition {
            name: "db_schema".into(),
            description: format!(
                "Discover database structure: list tables, describe columns, find relationships. \
                Use this FIRST before writing SQL queries with db_query. \
                This helps you understand what data is available.\n\n\
                Actions:\n\
                - 'tables' — List all tables in the database\n\
                - 'describe' — Show columns, types, and keys for a specific table\n\
                - 'sample' — Show 5 sample rows from a table\n\
                - 'summary' — Overview: all tables with row counts and column counts\n\n\
                Available connections:\n{}",
                conn_list
            ),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "connection_id": {
                        "type": "string",
                        "description": "ID of a pre-configured database connection"
                    },
                    "action": {
                        "type": "string",
                        "enum": ["tables", "describe", "sample", "summary"],
                        "description": "What to discover: 'tables' (list all), 'describe' (columns of a table), 'sample' (5 rows), 'summary' (tables + row counts)"
                    },
                    "table_name": {
                        "type": "string",
                        "description": "Table name (required for 'describe' and 'sample' actions)"
                    }
                },
                "required": ["connection_id", "action"]
            }),
        }
    }

    async fn execute(&self, arguments: &str) -> Result<ToolResult> {
        let args: serde_json::Value = serde_json::from_str(arguments)
            .map_err(|e| bizclaw_core::error::BizClawError::Tool(format!("Invalid JSON: {}", e)))?;

        let conn_id = args["connection_id"].as_str().ok_or_else(|| {
            bizclaw_core::error::BizClawError::Tool("Missing 'connection_id'".into())
        })?;

        let action = args["action"]
            .as_str()
            .ok_or_else(|| bizclaw_core::error::BizClawError::Tool("Missing 'action'".into()))?;

        // Resolve connection
        let profile = self.connection_manager.get(conn_id).ok_or_else(|| {
            let available = self
                .connection_manager
                .list()
                .iter()
                .map(|c| c.id.clone())
                .collect::<Vec<_>>()
                .join(", ");
            bizclaw_core::error::BizClawError::Tool(format!(
                "Unknown connection_id '{}'. Available: [{}]",
                conn_id, available
            ))
        })?;

        // Check table allowlist for describe/sample
        if let Some(table) = args["table_name"].as_str()
            && !profile.allowed_tables.is_empty()
            && !profile
                .allowed_tables
                .iter()
                .any(|t| t.eq_ignore_ascii_case(table))
        {
            return Ok(ToolResult {
                tool_call_id: String::new(),
                output: format!(
                    "🛡️ Table '{}' is not in the allowlist. Allowed: {:?}",
                    table, profile.allowed_tables
                ),
                success: false,
            });
        }

        let vault = bizclaw_security::vault::Vault::new();
        let uri =
            DbConnectionManager::resolve_connection_string(&profile.connection_string, &vault);

        // Connect
        let pool = AnyPoolOptions::new()
            .max_connections(2)
            .acquire_timeout(std::time::Duration::from_secs(10))
            .connect(&uri)
            .await
            .map_err(|e| {
                bizclaw_core::error::BizClawError::Tool(format!("Connection failed: {}", e))
            })?;

        let db_type = profile.db_type.to_lowercase();
        let start = std::time::Instant::now();

        let output = match action {
            "tables" => self.list_tables(&pool, &db_type).await,
            "describe" => {
                let table = args["table_name"].as_str().ok_or_else(|| {
                    bizclaw_core::error::BizClawError::Tool(
                        "'table_name' is required for 'describe'".into(),
                    )
                })?;
                self.describe_table(&pool, &db_type, table).await
            }
            "sample" => {
                let table = args["table_name"].as_str().ok_or_else(|| {
                    bizclaw_core::error::BizClawError::Tool(
                        "'table_name' is required for 'sample'".into(),
                    )
                })?;
                self.sample_table(&pool, table, profile).await
            }
            "summary" => self.db_summary(&pool, &db_type, profile).await,
            _ => Ok(format!(
                "❌ Unknown action '{}'. Use: tables, describe, sample, summary",
                action
            )),
        }?;

        let elapsed = start.elapsed();
        Ok(ToolResult {
            tool_call_id: String::new(),
            output: format!(
                "📊 [{}] ({}) — {}ms\n\n{}",
                conn_id,
                action,
                elapsed.as_millis(),
                output
            ),
            success: true,
        })
    }
}

impl DbSchemaTool {
    /// List all tables in the database.
    async fn list_tables(&self, pool: &sqlx::AnyPool, db_type: &str) -> Result<String> {
        let query = match db_type {
            "postgresql" | "postgres" => {
                "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' ORDER BY table_name"
            }
            "mysql" => "SHOW TABLES",
            "sqlite" => {
                "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name"
            }
            _ => {
                "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' ORDER BY table_name"
            }
        };

        let rows = sqlx::query(query)
            .fetch_all(pool)
            .await
            .map_err(|e| bizclaw_core::error::BizClawError::Tool(format!("Query failed: {}", e)))?;

        let mut tables = Vec::new();
        for row in &rows {
            if let Ok(name) = row.try_get::<String, _>(0) {
                tables.push(name);
            } else if let Ok(name) = row.try_get::<&str, _>(0) {
                tables.push(name.to_string());
            }
        }

        Ok(format!(
            "📋 Found {} tables:\n{}",
            tables.len(),
            tables
                .iter()
                .enumerate()
                .map(|(i, t)| format!("  {}. {}", i + 1, t))
                .collect::<Vec<_>>()
                .join("\n")
        ))
    }

    /// Describe columns of a table.
    async fn describe_table(
        &self,
        pool: &sqlx::AnyPool,
        db_type: &str,
        table: &str,
    ) -> Result<String> {
        let query = match db_type {
            "postgresql" | "postgres" => {
                format!(
                    "SELECT column_name, data_type, is_nullable, column_default \
                     FROM information_schema.columns \
                     WHERE table_name = '{}' AND table_schema = 'public' \
                     ORDER BY ordinal_position",
                    table.replace('\'', "''")
                )
            }
            "mysql" => format!("DESCRIBE `{}`", table.replace('`', "``")),
            "sqlite" => format!("PRAGMA table_info('{}')", table.replace('\'', "''")),
            _ => format!(
                "SELECT column_name, data_type, is_nullable FROM information_schema.columns WHERE table_name = '{}'",
                table.replace('\'', "''")
            ),
        };

        let rows = sqlx::query(&query)
            .fetch_all(pool)
            .await
            .map_err(|e| bizclaw_core::error::BizClawError::Tool(format!("Query failed: {}", e)))?;

        let mut cols = Vec::new();
        for row in &rows {
            let mut parts = Vec::new();
            for col in row.columns() {
                let val = match row.try_get_raw(col.name()) {
                    Ok(raw) if raw.is_null() => "NULL".to_string(),
                    Ok(_) => row
                        .try_get::<String, _>(col.name())
                        .or_else(|_| row.try_get::<&str, _>(col.name()).map(|s| s.to_string()))
                        .or_else(|_| row.try_get::<i64, _>(col.name()).map(|i| i.to_string()))
                        .or_else(|_| row.try_get::<i32, _>(col.name()).map(|i| i.to_string()))
                        .unwrap_or_else(|_| "?".to_string()),
                    Err(_) => "ERR".to_string(),
                };
                parts.push(format!("{}={}", col.name(), val));
            }
            cols.push(format!("  • {}", parts.join(" | ")));
        }

        Ok(format!(
            "🔍 Table '{}' — {} columns:\n{}",
            table,
            cols.len(),
            cols.join("\n")
        ))
    }

    /// Sample 5 rows from a table.
    async fn sample_table(
        &self,
        pool: &sqlx::AnyPool,
        table: &str,
        profile: &crate::db_connection::DbConnectionProfile,
    ) -> Result<String> {
        // Safe: we only SELECT LIMIT 5
        let query = format!("SELECT * FROM {} LIMIT 5", table);

        let rows = sqlx::query(&query)
            .fetch_all(pool)
            .await
            .map_err(|e| bizclaw_core::error::BizClawError::Tool(format!("Query failed: {}", e)))?;

        let mut results = Vec::new();
        for row in &rows {
            let mut row_map = serde_json::Map::new();
            for col in row.columns() {
                let name = col.name();
                let val = match row.try_get_raw(name) {
                    Ok(raw) if raw.is_null() => "NULL".to_string(),
                    Ok(_) => row
                        .try_get::<String, _>(name)
                        .or_else(|_| row.try_get::<&str, _>(name).map(|s| s.to_string()))
                        .or_else(|_| row.try_get::<i64, _>(name).map(|i| i.to_string()))
                        .or_else(|_| row.try_get::<i32, _>(name).map(|i| i.to_string()))
                        .or_else(|_| row.try_get::<f64, _>(name).map(|f| f.to_string()))
                        .or_else(|_| row.try_get::<bool, _>(name).map(|b| b.to_string()))
                        .unwrap_or_else(|_| "[Binary]".to_string()),
                    Err(_) => "ERR".to_string(),
                };
                row_map.insert(name.to_string(), serde_json::Value::String(val));
            }
            results.push(serde_json::Value::Object(row_map));
        }

        // Redact sensitive columns
        if !profile.sensitive_columns.is_empty() {
            DbConnectionManager::redact_sensitive(&mut results, &profile.sensitive_columns);
        }

        Ok(format!(
            "📝 Sample {} rows from '{}':\n{}",
            results.len(),
            table,
            serde_json::to_string_pretty(&results).unwrap_or_else(|_| "[]".into())
        ))
    }

    /// Database summary: all tables with row counts.
    async fn db_summary(
        &self,
        pool: &sqlx::AnyPool,
        db_type: &str,
        profile: &crate::db_connection::DbConnectionProfile,
    ) -> Result<String> {
        // First get tables
        let table_query = match db_type {
            "postgresql" | "postgres" => {
                "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' ORDER BY table_name"
            }
            "mysql" => "SHOW TABLES",
            "sqlite" => {
                "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name"
            }
            _ => {
                "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' ORDER BY table_name"
            }
        };

        let table_rows = sqlx::query(table_query)
            .fetch_all(pool)
            .await
            .map_err(|e| bizclaw_core::error::BizClawError::Tool(format!("Query failed: {}", e)))?;

        let mut tables = Vec::new();
        for row in &table_rows {
            if let Ok(name) = row.try_get::<String, _>(0) {
                tables.push(name);
            } else if let Ok(name) = row.try_get::<&str, _>(0) {
                tables.push(name.to_string());
            }
        }

        // Filter by allowlist if configured
        if !profile.allowed_tables.is_empty() {
            tables.retain(|t| {
                profile
                    .allowed_tables
                    .iter()
                    .any(|a| a.eq_ignore_ascii_case(t))
            });
        }

        // Get row counts for each table
        let mut summary_lines = Vec::new();
        for table in &tables {
            let count_query = format!("SELECT COUNT(*) FROM {}", table);
            let count = match sqlx::query(&count_query).fetch_one(pool).await {
                Ok(row) => row.try_get::<i64, _>(0).unwrap_or(0),
                Err(_) => -1,
            };

            // Get column count
            let col_query = match db_type {
                "postgresql" | "postgres" => format!(
                    "SELECT COUNT(*) FROM information_schema.columns WHERE table_name = '{}' AND table_schema = 'public'",
                    table.replace('\'', "''")
                ),
                "mysql" => format!(
                    "SELECT COUNT(*) FROM information_schema.columns WHERE table_name = '{}'",
                    table.replace('\'', "''")
                ),
                _ => format!(
                    "SELECT COUNT(*) FROM pragma_table_info('{}')",
                    table.replace('\'', "''")
                ),
            };

            let col_count = match sqlx::query(&col_query).fetch_one(pool).await {
                Ok(row) => row.try_get::<i64, _>(0).unwrap_or(0),
                Err(_) => 0,
            };

            summary_lines.push(format!(
                "  📋 {} — {} rows, {} columns",
                table,
                if count >= 0 {
                    count.to_string()
                } else {
                    "?".to_string()
                },
                col_count
            ));
        }

        Ok(format!(
            "📊 Database Summary — {} tables:\n\n{}",
            tables.len(),
            summary_lines.join("\n")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_name() {
        let tool = DbSchemaTool::new();
        assert_eq!(tool.name(), "db_schema");
    }

    #[test]
    fn test_definition_has_actions() {
        let tool = DbSchemaTool::new();
        let def = tool.definition();
        let desc = def.description;
        assert!(desc.contains("tables"));
        assert!(desc.contains("describe"));
        assert!(desc.contains("sample"));
        assert!(desc.contains("summary"));
    }
}
