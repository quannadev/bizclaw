//! Natural Language to SQL Query Tool — RAG-powered Text2SQL pipeline
//!
//! 6-step Text2SQL pipeline:
//! 1. Normalize question (remove specifics, keep intent)
//! 2. Select tables (semantic layer + LLM reasoning)
//! 3. Find similar examples (FTS5 few-shot matching)
//! 4. Generate SQL (LLM with schema + examples + business rules)
//! 5. Validate & execute (safety check + run)
//! 6. Learn (save successful Q&A for future)
//!
//! ```text
//! User: "Doanh thu tháng này bao nhiêu?"
//!   ↓ normalize → "doanh thu theo tháng"
//!   ↓ select tables → [orders]
//!   ↓ find examples → "doanh thu tháng trước" → SELECT SUM(total)...
//!   ↓ generate SQL → SELECT SUM(total) FROM orders WHERE ...
//!   ↓ execute → 280,000,000
//!   ↓ learn → save {question, sql} for next time
//! ```

use crate::db_connection::DbConnectionManager;
use crate::db_examples::{BusinessRuleStore, SqlExampleStore};
use crate::db_semantic::{SchemaLayerStore, SchemaPrompts};
use async_trait::async_trait;
use bizclaw_core::error::Result;
use bizclaw_core::traits::Tool;
use bizclaw_core::types::{ToolDefinition, ToolResult};
use sqlx::Row;
use sqlx::any::AnyPoolOptions;

pub struct NlQueryTool {
    connection_manager: DbConnectionManager,
    example_store: SqlExampleStore,
    rule_store: BusinessRuleStore,
    schema_store: SchemaLayerStore,
}

impl NlQueryTool {
    pub fn new() -> Self {
        sqlx::any::install_default_drivers();
        Self {
            connection_manager: DbConnectionManager::load_default(),
            example_store: SqlExampleStore::default(),
            rule_store: BusinessRuleStore::default(),
            schema_store: SchemaLayerStore::default(),
        }
    }
}

impl Default for NlQueryTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for NlQueryTool {
    fn name(&self) -> &str {
        "nl_query"
    }

    fn definition(&self) -> ToolDefinition {
        let connections = self.connection_manager.list();
        let conn_list = if connections.is_empty() {
            "No connections configured.".to_string()
        } else {
            connections
                .iter()
                .map(|c| format!("  - '{}' ({}) — {}", c.id, c.db_type, c.description))
                .collect::<Vec<_>>()
                .join("\n")
        };

        let example_count = self.example_store.count();
        let indexed = self.schema_store.list_indexed();

        ToolDefinition {
            name: "nl_query".into(),
            description: format!(
                "Ask database questions in natural language. AI automatically:\n\
                 1. Understands your question\n\
                 2. Selects relevant tables\n\
                 3. Generates SQL query\n\
                 4. Executes safely (read-only)\n\
                 5. Returns formatted results\n\n\
                 Actions:\n\
                 - 'ask' — Ask a question in natural language\n\
                 - 'index' — Index database schema (run once per connection)\n\
                 - 'add_rule' — Add a business rule\n\
                 - 'list_rules' — List business rules\n\
                 - 'list_examples' — Show learned Q&A pairs\n\
                 - 'save_example' — Manually save a Q&A pair\n\n\
                 Available connections:\n{}\n\n\
                 Schema indexed: [{}]\n\
                 Learned examples: {}",
                conn_list,
                indexed.join(", "),
                example_count,
            ),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["ask", "index", "add_rule", "list_rules", "list_examples", "save_example"],
                        "description": "What to do"
                    },
                    "connection_id": {
                        "type": "string",
                        "description": "Database connection ID"
                    },
                    "question": {
                        "type": "string",
                        "description": "Natural language question (for 'ask' action)"
                    },
                    "rule": {
                        "type": "string",
                        "description": "Business rule text (for 'add_rule' action)"
                    },
                    "sql": {
                        "type": "string",
                        "description": "SQL query (for 'save_example' action)"
                    }
                },
                "required": ["action", "connection_id"]
            }),
        }
    }

    async fn execute(&self, arguments: &str) -> Result<ToolResult> {
        let args: serde_json::Value = serde_json::from_str(arguments)
            .map_err(|e| bizclaw_core::error::BizClawError::Tool(format!("Invalid JSON: {e}")))?;

        let action = args["action"].as_str().unwrap_or("ask");
        let conn_id = args["connection_id"].as_str().unwrap_or("");

        match action {
            "ask" => {
                let question = args["question"].as_str().unwrap_or("");
                if question.is_empty() {
                    return Ok(ToolResult {
                        tool_call_id: String::new(),
                        output: "❌ Missing 'question' parameter".into(),
                        success: false,
                    });
                }
                self.ask_question(question, conn_id).await
            }
            "index" => self.index_schema(conn_id).await,
            "add_rule" => {
                let rule = args["rule"].as_str().unwrap_or("");
                self.add_rule(conn_id, rule)
            }
            "list_rules" => self.list_rules(conn_id),
            "list_examples" => self.list_examples(conn_id),
            "save_example" => {
                let question = args["question"].as_str().unwrap_or("");
                let sql = args["sql"].as_str().unwrap_or("");
                self.save_example(conn_id, question, sql)
            }
            _ => Ok(ToolResult {
                tool_call_id: String::new(),
                output: format!("❌ Unknown action: {action}"),
                success: false,
            }),
        }
    }
}

impl NlQueryTool {
    /// Main pipeline: NL question → SQL → results.
    async fn ask_question(&self, question: &str, conn_id: &str) -> Result<ToolResult> {
        let profile = self.connection_manager.get(conn_id).ok_or_else(|| {
            bizclaw_core::error::BizClawError::Tool(format!("Unknown connection: {conn_id}"))
        })?;

        let start = std::time::Instant::now();

        // Step 1: Check if schema is indexed
        if !self.schema_store.is_indexed(conn_id) {
            return Ok(ToolResult {
                tool_call_id: String::new(),
                output: format!(
                    "⚠️ Schema not indexed for '{conn_id}'. Run nl_query with action='index' first.\n\
                     This creates a semantic understanding of your database structure."
                ),
                success: false,
            });
        }

        // Step 2: Get schema docs for table selection
        let schema_docs = self.schema_store.format_tables_for_selection(conn_id);

        // Step 3: Get business rules
        let business_rules = self.rule_store.format_for_prompt(conn_id);

        // Step 4: Find similar examples (few-shot)
        let examples = self.example_store.find_similar(question, conn_id, 5);
        let examples_text = if examples.is_empty() {
            "No similar examples found.".to_string()
        } else {
            examples
                .iter()
                .map(|e| format!("Q: {}\nSQL: {}", e.question, e.sql))
                .collect::<Vec<_>>()
                .join("\n\n")
        };

        // Step 5: Build the SQL generation prompt
        // (In a full implementation, we'd call LLM for table selection first,
        //  then generate SQL. For now, we combine into a single prompt.)
        let db_type = &profile.db_type;
        let prompt = SchemaPrompts::sql_generation_prompt(
            question,
            &schema_docs,
            &examples_text,
            &business_rules,
            db_type,
        );

        // Step 6: Since we can't call LLM directly from a tool,
        // we return the prompt context for the agent to use.
        // The agent's LLM will generate SQL based on this.
        let detail = format!(
            "🧠 NL Query Pipeline for: \"{question}\"\n\
             📊 Connection: {conn_id} ({db_type})\n\
             📋 Schema: {} tables indexed\n\
             📝 Examples: {} similar found\n\
             📏 Rules: {} active\n\
             ⏱️ Prep: {}ms\n\n\
             ---\n\
             The following context has been prepared. Generate a SQL query to answer the question.\n\n\
             {prompt}\n\n\
             ---\n\
             After generating SQL, use the `db_query` tool to execute it against connection '{conn_id}'.\n\
             If the query works correctly, use `nl_query` with action='save_example' to save the Q&A pair for future learning.",
            self.schema_store.table_names(conn_id).len(),
            examples.len(),
            self.rule_store.get_rules(conn_id).len(),
            start.elapsed().as_millis(),
        );

        Ok(ToolResult {
            tool_call_id: String::new(),
            output: detail,
            success: true,
        })
    }

    /// Index a database schema into the semantic layer.
    async fn index_schema(&self, conn_id: &str) -> Result<ToolResult> {
        let profile = self.connection_manager.get(conn_id).ok_or_else(|| {
            bizclaw_core::error::BizClawError::Tool(format!("Unknown connection: {conn_id}"))
        })?;

        let vault = bizclaw_security::vault::Vault::new();
        let uri =
            DbConnectionManager::resolve_connection_string(&profile.connection_string, &vault);

        let pool = AnyPoolOptions::new()
            .max_connections(2)
            .acquire_timeout(std::time::Duration::from_secs(15))
            .connect(&uri)
            .await
            .map_err(|e| {
                bizclaw_core::error::BizClawError::Tool(format!("Connection failed: {e}"))
            })?;

        let db_type = profile.db_type.to_lowercase();
        let start = std::time::Instant::now();

        // Get all tables
        let table_query = match db_type.as_str() {
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
            .fetch_all(&pool)
            .await
            .map_err(|e| bizclaw_core::error::BizClawError::Tool(format!("Query failed: {e}")))?;

        let mut table_names = Vec::new();
        for row in &table_rows {
            if let Ok(name) = row.try_get::<String, _>(0) {
                table_names.push(name);
            } else if let Ok(name) = row.try_get::<&str, _>(0) {
                table_names.push(name.to_string());
            }
        }

        // Filter by allowlist
        if !profile.allowed_tables.is_empty() {
            table_names.retain(|t| {
                profile
                    .allowed_tables
                    .iter()
                    .any(|a| a.eq_ignore_ascii_case(t))
            });
        }

        // For each table, get columns + row count → build raw schema doc
        let mut table_docs = Vec::new();
        let mut raw_schemas = Vec::new();

        for table in &table_names {
            // Get columns
            let col_query = match db_type.as_str() {
                "postgresql" | "postgres" => format!(
                    "SELECT column_name, data_type, is_nullable, column_default \
                     FROM information_schema.columns \
                     WHERE table_name = '{}' AND table_schema = 'public' \
                     ORDER BY ordinal_position",
                    table.replace('\'', "''")
                ),
                "mysql" => format!("DESCRIBE `{}`", table.replace('`', "``")),
                "sqlite" => format!("PRAGMA table_info('{}')", table.replace('\'', "''")),
                _ => format!(
                    "SELECT column_name, data_type, is_nullable FROM information_schema.columns WHERE table_name = '{}'",
                    table.replace('\'', "''")
                ),
            };

            let col_rows = match sqlx::query(&col_query).fetch_all(&pool).await {
                Ok(r) => r,
                Err(_) => continue,
            };

            let mut columns = Vec::new();
            let mut raw_cols = Vec::new();
            for row in &col_rows {
                let col_name = row
                    .try_get::<String, _>(0)
                    .or_else(|_| row.try_get::<&str, _>(0).map(|s| s.to_string()))
                    .unwrap_or_default();
                let data_type = row
                    .try_get::<String, _>(1)
                    .or_else(|_| row.try_get::<&str, _>(1).map(|s| s.to_string()))
                    .unwrap_or("unknown".to_string());
                let nullable_str = row
                    .try_get::<String, _>(2)
                    .or_else(|_| row.try_get::<&str, _>(2).map(|s| s.to_string()))
                    .unwrap_or("YES".to_string());
                let nullable = nullable_str.to_uppercase() == "YES" || nullable_str == "1";

                raw_cols.push(format!(
                    "  {} {} {}",
                    col_name,
                    data_type,
                    if nullable { "NULL" } else { "NOT NULL" }
                ));

                // Auto-detect keys and generate basic description
                let _is_key = col_name.ends_with("_id") || col_name == "id";
                let description = if col_name == "id" {
                    "Primary key".to_string()
                } else if col_name.ends_with("_id") {
                    let ref_table = col_name.trim_end_matches("_id");
                    format!("Foreign key → {ref_table}s table")
                } else if col_name.contains("created") || col_name.contains("updated") {
                    "Timestamp".to_string()
                } else if col_name.contains("name") || col_name.contains("title") {
                    "Name/title field".to_string()
                } else if col_name.contains("email") {
                    "Email address".to_string()
                } else if col_name.contains("phone") {
                    "Phone number".to_string()
                } else if col_name.contains("total")
                    || col_name.contains("amount")
                    || col_name.contains("price")
                {
                    "Monetary value".to_string()
                } else if col_name.contains("status") || col_name.contains("state") {
                    "Status/state field".to_string()
                } else if col_name.contains("count") || col_name.contains("quantity") {
                    "Quantity/count".to_string()
                } else {
                    format!("{} field", data_type)
                };

                columns.push(crate::db_semantic::ColumnDoc {
                    name: col_name,
                    data_type,
                    nullable,
                    description,
                });
            }

            // Get row count
            let count_query = format!("SELECT COUNT(*) FROM {table}");
            let row_count = match sqlx::query(&count_query).fetch_one(&pool).await {
                Ok(row) => row.try_get::<i64, _>(0).unwrap_or(0),
                Err(_) => 0,
            };

            // Auto-detect keys and connected tables
            let keys: Vec<String> = columns
                .iter()
                .filter(|c| c.name.ends_with("_id") && c.name != "id")
                .map(|c| c.name.clone())
                .collect();

            let connected: Vec<String> = keys
                .iter()
                .map(|k| {
                    let base = k.trim_end_matches("_id");
                    format!("{base}s") // Simple pluralization
                })
                .collect();

            // Auto-detect entities
            let entities: Vec<String> = self.infer_entities(table, &columns);

            raw_schemas.push(format!("TABLE: {table}\n{}", raw_cols.join("\n")));

            table_docs.push(crate::db_semantic::TableDoc {
                name: table.clone(),
                summary: format!("Table with {} columns, {} rows", columns.len(), row_count),
                purpose: String::new(), // Will be enriched by LLM if available
                row_count,
                columns,
                keys,
                connected_tables: connected,
                entities,
            });
        }

        // Save the semantic layer
        let layer = crate::db_semantic::SchemaSemanticLayer {
            connection_id: conn_id.to_string(),
            db_type: db_type.clone(),
            indexed_at: chrono::Utc::now().to_rfc3339(),
            tables: table_docs,
        };

        self.schema_store
            .save(&layer)
            .map_err(bizclaw_core::error::BizClawError::Tool)?;

        let elapsed = start.elapsed();

        // Also prepare the raw schemas for LLM enrichment
        let _raw_schema_text = raw_schemas.join("\n\n");

        Ok(ToolResult {
            tool_call_id: String::new(),
            output: format!(
                "✅ Schema indexed for '{conn_id}' ({db_type})\n\
                 📊 {} tables indexed in {}ms\n\
                 📋 Tables: {}\n\n\
                 The basic schema has been indexed with auto-detected keys, relationships, and entities.\n\n\
                 To enrich the documentation with AI-generated descriptions, you can process each table's \
                 raw schema through an LLM using the following prompt template:\n\n\
                 ---\n{}\n---\n\n\
                 This is optional — the basic index is already usable for NL queries.",
                table_names.len(),
                elapsed.as_millis(),
                table_names.join(", "),
                crate::db_semantic::SchemaPrompts::process_table_prompt(
                    raw_schemas.first().unwrap_or(&String::new())
                ),
            ),
            success: true,
        })
    }

    /// Infer business entities from table name and columns.
    fn infer_entities(
        &self,
        table: &str,
        columns: &[crate::db_semantic::ColumnDoc],
    ) -> Vec<String> {
        let mut entities = Vec::new();

        // Based on monetary columns
        for col in columns {
            if col.name.contains("total")
                || col.name.contains("amount")
                || col.name.contains("price")
            {
                entities.push(format!("{} {}", table, col.name));
            }
        }

        // Based on table name patterns
        let t = table.to_lowercase();
        if t.contains("order") {
            entities.push("total orders".to_string());
            entities.push("revenue".to_string());
        } else if t.contains("customer") || t.contains("user") {
            entities.push("customer count".to_string());
            entities.push("active users".to_string());
        } else if t.contains("product") || t.contains("item") {
            entities.push("product catalog".to_string());
        } else if t.contains("invoice") || t.contains("payment") {
            entities.push("payment total".to_string());
        }

        if entities.is_empty() {
            entities.push(format!("{table} records"));
        }

        entities
    }

    /// Add a business rule.
    fn add_rule(&self, conn_id: &str, rule: &str) -> Result<ToolResult> {
        if rule.is_empty() {
            return Ok(ToolResult {
                tool_call_id: String::new(),
                output: "❌ Missing 'rule' parameter".into(),
                success: false,
            });
        }

        match self.rule_store.add_rule(conn_id, rule) {
            Ok(id) => Ok(ToolResult {
                tool_call_id: String::new(),
                output: format!("✅ Business rule added: {id}\nRule: {rule}\nFor: {conn_id}"),
                success: true,
            }),
            Err(e) => Ok(ToolResult {
                tool_call_id: String::new(),
                output: format!("❌ Failed to add rule: {e}"),
                success: false,
            }),
        }
    }

    /// List business rules.
    fn list_rules(&self, conn_id: &str) -> Result<ToolResult> {
        let rules = self.rule_store.get_rules(conn_id);
        if rules.is_empty() {
            return Ok(ToolResult {
                tool_call_id: String::new(),
                output: format!(
                    "📋 No business rules for '{conn_id}'. Add with action='add_rule'."
                ),
                success: true,
            });
        }

        let lines: Vec<String> = rules
            .iter()
            .map(|r| format!("  [{}/{}] {}", r.id, r.connection_id, r.rule))
            .collect();

        Ok(ToolResult {
            tool_call_id: String::new(),
            output: format!("📋 Business rules for '{conn_id}':\n{}", lines.join("\n")),
            success: true,
        })
    }

    /// List learned examples.
    fn list_examples(&self, conn_id: &str) -> Result<ToolResult> {
        let examples = self.example_store.list_recent(conn_id, 20);
        if examples.is_empty() {
            return Ok(ToolResult {
                tool_call_id: String::new(),
                output: format!(
                    "📝 No learned examples for '{conn_id}'. Examples are saved when NL queries succeed."
                ),
                success: true,
            });
        }

        let lines: Vec<String> = examples
            .iter()
            .map(|e| {
                format!(
                    "  [{}] Q: {}\n       SQL: {}",
                    e.id,
                    if e.question.len() > 60 {
                        format!("{}...", &e.question.chars().take(60).collect::<String>())
                    } else {
                        e.question.clone()
                    },
                    if e.sql.len() > 80 {
                        format!("{}...", &e.sql.chars().take(80).collect::<String>())
                    } else {
                        e.sql.clone()
                    },
                )
            })
            .collect();

        Ok(ToolResult {
            tool_call_id: String::new(),
            output: format!(
                "📝 Learned examples for '{}' ({} total):\n{}",
                conn_id,
                examples.len(),
                lines.join("\n")
            ),
            success: true,
        })
    }

    /// Save a Q&A example manually.
    fn save_example(&self, conn_id: &str, question: &str, sql: &str) -> Result<ToolResult> {
        if question.is_empty() || sql.is_empty() {
            return Ok(ToolResult {
                tool_call_id: String::new(),
                output: "❌ Both 'question' and 'sql' are required".into(),
                success: false,
            });
        }

        // Extract table names from SQL (simple heuristic)
        let tables = Self::extract_tables_from_sql(sql);

        // Create a normalized version (simple: lowercase, remove numbers)
        let normalized = question
            .to_lowercase()
            .chars()
            .filter(|c| c.is_alphabetic() || c.is_whitespace())
            .collect::<String>();

        match self
            .example_store
            .save(question, &normalized, sql, conn_id, &tables)
        {
            Ok(id) => Ok(ToolResult {
                tool_call_id: String::new(),
                output: format!(
                    "✅ Example saved (ID: {id})\nQ: {question}\nSQL: {sql}\nTables: {}",
                    tables.join(", ")
                ),
                success: true,
            }),
            Err(e) => Ok(ToolResult {
                tool_call_id: String::new(),
                output: format!("❌ Failed to save: {e}"),
                success: false,
            }),
        }
    }

    /// Extract table names from a SQL query (simple heuristic).
    fn extract_tables_from_sql(sql: &str) -> Vec<String> {
        let upper = sql.to_uppercase();
        let mut tables = Vec::new();

        for keyword in &["FROM", "JOIN"] {
            for part in upper.split(keyword).skip(1) {
                if let Some(table_name) = part.split_whitespace().next() {
                    let clean = table_name
                        .trim_matches(|c: char| c == '`' || c == '"' || c == '\'' || c == '(')
                        .to_lowercase();
                    if !clean.is_empty()
                        && ![
                            "where", "on", "as", "and", "or", "set", "left", "right", "inner",
                            "outer", "cross", "group", "order", "limit", "having", "union", "(",
                            "select",
                        ]
                        .contains(&clean.as_str())
                        && !tables.contains(&clean)
                    {
                        tables.push(clean);
                    }
                }
            }
        }

        tables
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_name() {
        let tool = NlQueryTool::new();
        assert_eq!(tool.name(), "nl_query");
    }

    #[test]
    fn test_definition() {
        let tool = NlQueryTool::new();
        let def = tool.definition();
        assert!(def.description.contains("natural language"));
        assert!(def.description.contains("ask"));
        assert!(def.description.contains("index"));
    }

    #[test]
    fn test_extract_tables() {
        let tables = NlQueryTool::extract_tables_from_sql(
            "SELECT o.total, c.name FROM orders o JOIN customers c ON o.customer_id = c.id WHERE o.status = 'completed'",
        );
        assert!(tables.contains(&"orders".to_string()));
        assert!(tables.contains(&"customers".to_string()));
    }

    #[test]
    fn test_extract_tables_complex() {
        let tables = NlQueryTool::extract_tables_from_sql(
            "SELECT p.name, SUM(oi.quantity) FROM products p \
             LEFT JOIN order_items oi ON p.id = oi.product_id \
             JOIN orders o ON oi.order_id = o.id \
             GROUP BY p.name",
        );
        assert!(tables.contains(&"products".to_string()));
        assert!(tables.contains(&"order_items".to_string()));
        assert!(tables.contains(&"orders".to_string()));
    }
}
