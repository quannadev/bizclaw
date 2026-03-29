//! Database Schema Semantic Layer — Auto-generate rich docs from raw schema
//!
//! Instant documentation for database columns.
//! Connects to DB → reads schema → stores structured JSON docs in knowledge base.
//!
//! The semantic layer enables:
//! - Table selection via natural language (which tables answer "doanh thu"?)
//! - Column understanding (what does `total` mean in `orders`?)
//! - Relationship mapping (how do `orders` and `customers` connect?)

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Structured description of a single database table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableDoc {
    pub name: String,
    pub summary: String,
    pub purpose: String,
    pub row_count: i64,
    pub columns: Vec<ColumnDoc>,
    pub keys: Vec<String>,
    pub connected_tables: Vec<String>,
    pub entities: Vec<String>,
}

/// Structured description of a column.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnDoc {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub description: String,
}

/// Semantic layer for a database connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaSemanticLayer {
    pub connection_id: String,
    pub db_type: String,
    pub indexed_at: String,
    pub tables: Vec<TableDoc>,
}

/// Store for schema semantic layers.
pub struct SchemaLayerStore {
    data_dir: PathBuf,
}

impl SchemaLayerStore {
    pub fn new(data_dir: &Path) -> Self {
        let dir = data_dir.join("schema-layers");
        let _ = std::fs::create_dir_all(&dir);
        Self { data_dir: dir }
    }

    pub fn default() -> Self {
        Self::new(Path::new("data"))
    }

    /// Save a semantic layer for a connection.
    pub fn save(&self, layer: &SchemaSemanticLayer) -> Result<(), String> {
        let path = self.data_dir.join(format!("{}.json", layer.connection_id));
        let json = serde_json::to_string_pretty(layer).map_err(|e| format!("Serialize: {e}"))?;
        std::fs::write(&path, json).map_err(|e| format!("Write: {e}"))?;
        tracing::info!(
            "[schema-layer] Saved {} tables for '{}'",
            layer.tables.len(),
            layer.connection_id
        );
        Ok(())
    }

    /// Load a semantic layer for a connection.
    pub fn load(&self, connection_id: &str) -> Option<SchemaSemanticLayer> {
        let path = self.data_dir.join(format!("{}.json", connection_id));
        let content = std::fs::read_to_string(&path).ok()?;
        serde_json::from_str(&content).ok()
    }

    /// Check if a semantic layer exists and is recent.
    pub fn is_indexed(&self, connection_id: &str) -> bool {
        self.load(connection_id).is_some()
    }

    /// Format table descriptions for LLM prompt (table selection step).
    pub fn format_tables_for_selection(&self, connection_id: &str) -> String {
        let layer = match self.load(connection_id) {
            Some(l) => l,
            None => return "⚠️ No schema indexed. Run `db_schema index` first.".to_string(),
        };

        let mut lines = Vec::new();
        for table in &layer.tables {
            let col_names: Vec<&str> = table.columns.iter().map(|c| c.name.as_str()).collect();
            lines.push(format!(
                "## {}\n{}\nPurpose: {}\nColumns: {}\nKeys: {}\nConnects to: {}\nEntities: {}\nRows: {}",
                table.name,
                table.summary,
                table.purpose,
                col_names.join(", "),
                table.keys.join(", "),
                table.connected_tables.join(", "),
                table.entities.join(", "),
                table.row_count,
            ));
        }
        lines.join("\n\n")
    }

    /// Format specific tables for SQL generation prompt (detailed).
    pub fn format_tables_for_sql(&self, connection_id: &str, table_names: &[String]) -> String {
        let layer = match self.load(connection_id) {
            Some(l) => l,
            None => return String::new(),
        };

        let mut lines = Vec::new();
        for table in &layer.tables {
            if table_names
                .iter()
                .any(|n| n.eq_ignore_ascii_case(&table.name))
            {
                let cols: Vec<String> = table
                    .columns
                    .iter()
                    .map(|c| {
                        format!(
                            "  - {} ({}{}) — {}",
                            c.name,
                            c.data_type,
                            if c.nullable { ", nullable" } else { "" },
                            c.description
                        )
                    })
                    .collect();

                lines.push(format!(
                    "### TABLE: {}\n{}\nPurpose: {}\nKeys: [{}]\nConnected: [{}]\nColumns:\n{}",
                    table.name,
                    table.summary,
                    table.purpose,
                    table.keys.join(", "),
                    table.connected_tables.join(", "),
                    cols.join("\n"),
                ));
            }
        }
        lines.join("\n\n")
    }

    /// Get all table names from the semantic layer.
    pub fn table_names(&self, connection_id: &str) -> Vec<String> {
        match self.load(connection_id) {
            Some(l) => l.tables.iter().map(|t| t.name.clone()).collect(),
            None => vec![],
        }
    }

    /// Delete a semantic layer.
    pub fn delete(&self, connection_id: &str) -> Result<(), String> {
        let path = self.data_dir.join(format!("{}.json", connection_id));
        std::fs::remove_file(&path).map_err(|e| format!("Delete: {e}"))
    }

    /// List all indexed connections.
    pub fn list_indexed(&self) -> Vec<String> {
        let mut result = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&self.data_dir) {
            for entry in entries.flatten() {
                if let Some(name) = entry.path().file_stem().and_then(|s| s.to_str()) {
                    result.push(name.to_string());
                }
            }
        }
        result
    }
}

/// Prompt templates for schema indexing (ported from Text2SQL).
pub struct SchemaPrompts;

impl SchemaPrompts {
    /// Prompt to process a raw table schema into a structured document.
    /// Input: raw DDL / column info from information_schema.
    pub fn process_table_prompt(raw_schema: &str) -> String {
        format!(
            r#"Here is a database table schema:
#####
{raw_schema}
#####

Analyze this table and return a JSON object with this EXACT structure:
{{
  "name": "<table name>",
  "summary": "<1-2 sentence summary of what this table stores>",
  "purpose": "<what business purpose this table serves>",
  "keys": ["<list of foreign key columns, usually ending in _id>"],
  "connected_tables": ["<tables connected via foreign keys>"],
  "entities": ["<business entities derivable from this table, e.g. 'total revenue', 'customer count'>"],
  "columns": [
    {{"name": "<column>", "data_type": "<type>", "nullable": true/false, "description": "<what this column represents>"}}
  ]
}}

Rules:
1. Summary should be in Vietnamese if column/table names suggest a Vietnamese business context, otherwise English.
2. For entities: extract 3-5 business concepts (e.g. "monthly revenue", "active customers", "pending orders").
3. For connected_tables: infer from foreign key columns (e.g. customer_id → customers table).
4. Keep descriptions concise but informative.
5. Return ONLY valid JSON, no markdown, no explanation."#
        )
    }

    /// Prompt to select relevant tables for a NL question.
    pub fn table_selection_prompt(
        question: &str,
        table_descriptions: &str,
        business_rules: &str,
    ) -> String {
        format!(
            r#"Analyze the user question and determine which database tables are needed to answer it.

Question: "{question}"

Available tables:
{table_descriptions}

{business_rules}

Think step by step:
1. What data does the user want?
2. Which tables contain this data?
3. What JOINs are needed?

Return as JSON:
{{"tables": ["<list of table names needed>"]}}

Rules:
- Only include tables that are NECESSARY for the query
- Minimize the number of tables
- Consider foreign key relationships
- Return ONLY valid JSON"#
        )
    }

    /// Prompt to generate SQL from NL question with schema + examples.
    pub fn sql_generation_prompt(
        question: &str,
        schema_docs: &str,
        examples: &str,
        business_rules: &str,
        db_dialect: &str,
    ) -> String {
        format!(
            r#"Write a SQL query to answer: "{question}"

** DATABASE DIALECT: {db_dialect} **

** DATASETS (table descriptions) **
{schema_docs}

** SIMILAR EXAMPLES **
{examples}

** BUSINESS RULES **
{business_rules}

Instructions:
1. Use ONLY the tables described in DATASETS above
2. Follow the business rules strictly
3. Minimize the number of tables — avoid unnecessary JOINs
4. Before aggregations (SUM, AVG, COUNT), filter out NULL values
5. Use appropriate WHERE clauses for the question
6. Return ONLY the SQL query, no explanation, no markdown
7. The query MUST be read-only (SELECT only)

SQL:"#
        )
    }

    /// Prompt to fix a broken SQL query.
    pub fn fix_sql_prompt(sql: &str, error: &str, dialect: &str) -> String {
        format!(
            r#"Fix this SQL query ({dialect}):
```sql
{sql}
```

Error: "{error}"

Return ONLY the corrected SQL query, no explanation."#
        )
    }

    /// Prompt to normalize a user question (for better example matching).
    pub fn normalize_question_prompt(question: &str) -> String {
        format!(
            r#"Normalize this database question by removing specific values but keeping the intent:

Question: "{question}"

Rules:
1. Remove specific dates/numbers/names → replace with general terms
2. Remove words like "show", "display" → use directive form
3. Keep entity types (e.g. "customers", "orders", "revenue")
4. Keep important qualifiers (e.g. "active", "pending", "monthly")

Return ONLY the normalized question as a single string, no JSON, no explanation.
Example: "doanh thu tháng 3 năm 2024" → "doanh thu theo tháng""#
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_layer_store() {
        let dir = PathBuf::from("/tmp/bizclaw_test_schema_layer");
        let _ = std::fs::remove_dir_all(&dir);

        let store = SchemaLayerStore::new(&dir);

        let layer = SchemaSemanticLayer {
            connection_id: "test_pg".to_string(),
            db_type: "postgres".to_string(),
            indexed_at: "2026-03-13T09:00:00Z".to_string(),
            tables: vec![TableDoc {
                name: "orders".to_string(),
                summary: "Đơn hàng của khách".to_string(),
                purpose: "Lưu trữ đơn hàng".to_string(),
                row_count: 15000,
                columns: vec![
                    ColumnDoc {
                        name: "id".to_string(),
                        data_type: "integer".to_string(),
                        nullable: false,
                        description: "Primary key".to_string(),
                    },
                    ColumnDoc {
                        name: "total".to_string(),
                        data_type: "numeric".to_string(),
                        nullable: false,
                        description: "Tổng tiền đơn".to_string(),
                    },
                ],
                keys: vec!["customer_id".to_string()],
                connected_tables: vec!["customers".to_string()],
                entities: vec!["revenue".to_string(), "order count".to_string()],
            }],
        };

        // Save & Load
        store.save(&layer).unwrap();
        assert!(store.is_indexed("test_pg"));

        let loaded = store.load("test_pg").unwrap();
        assert_eq!(loaded.tables.len(), 1);
        assert_eq!(loaded.tables[0].name, "orders");

        // Format for prompt
        let selection = store.format_tables_for_selection("test_pg");
        assert!(selection.contains("orders"));
        assert!(selection.contains("Đơn hàng"));

        // Table names
        let names = store.table_names("test_pg");
        assert_eq!(names, vec!["orders"]);

        // List indexed
        let indexed = store.list_indexed();
        assert!(indexed.contains(&"test_pg".to_string()));

        // Delete
        store.delete("test_pg").unwrap();
        assert!(!store.is_indexed("test_pg"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_prompts() {
        let p = SchemaPrompts::process_table_prompt("CREATE TABLE orders (id INT, total NUMERIC)");
        assert!(p.contains("orders"));

        let p = SchemaPrompts::table_selection_prompt("doanh thu", "## orders\n...", "");
        assert!(p.contains("doanh thu"));

        let p = SchemaPrompts::sql_generation_prompt(
            "revenue",
            "docs",
            "examples",
            "rules",
            "postgresql",
        );
        assert!(p.contains("revenue"));
        assert!(p.contains("postgresql"));
    }
}
