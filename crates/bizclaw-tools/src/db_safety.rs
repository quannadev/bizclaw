//! Database Safety checking for Agent Queries

use regex::Regex;

pub struct DbSafety;

impl DbSafety {
    pub fn ensure_safe_query(query: &str) -> std::result::Result<(), String> {
        let q = query.trim().to_uppercase();

        // 1. Check blocked keywords (Basic SQL injection / dangerous query protection for agent context)
        let blocked = [
            "DROP",
            "TRUNCATE",
            "DELETE",
            "UPDATE",
            "INSERT",
            "ALTER",
            "CREATE",
            "GRANT",
            "REVOKE",
            "EXEC",
            "CALL",
            "LOAD",
            "INTO OUTFILE",
        ];

        for kw in &blocked {
            let pattern = format!(r"\b{}\b", kw);
            if let Ok(re) = Regex::new(&pattern)
                && re.is_match(&q)
            {
                return Err(format!("Blocked dangerous keyword: {}", kw));
            }
        }

        // 2. Check allowed starting keyword
        let first_word = q.split_whitespace().next().unwrap_or("");
        match first_word {
            "SELECT" | "SHOW" | "DESCRIBE" | "EXPLAIN" | "WITH" => Ok(()),
            _ => Err(format!(
                "Only SELECT, SHOW, DESCRIBE, EXPLAIN, WITH queries are allowed. Got: {}",
                first_word
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_db_safety_allowed() {
        assert!(DbSafety::ensure_safe_query("SELECT * FROM users").is_ok());
        assert!(DbSafety::ensure_safe_query("SHOW TABLES").is_ok());
        assert!(DbSafety::ensure_safe_query("WITH cte AS (SELECT 1) SELECT * FROM cte").is_ok());
        assert!(DbSafety::ensure_safe_query("   describe   products ").is_ok());
    }

    #[test]
    fn test_db_safety_blocked() {
        assert!(DbSafety::ensure_safe_query("DELETE FROM users").is_err());
        assert!(DbSafety::ensure_safe_query("SELECT * FROM users; DROP TABLE abc;").is_err());
        assert!(DbSafety::ensure_safe_query("INSERT INTO abc VALUES (1)").is_err());
    }
}
