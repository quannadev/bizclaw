---
name: sql-expert
description: |
  SQL expert for BizClaw database design, queries, and SQLite optimization. Trigger phrases:
  SQL query, database design, SQLite, PostgreSQL, query optimization, index, schema,
  migration, sqlx, ORM, transactions, join, aggregate, window function.
  Scenarios: khi cần viết SQL query, khi cần thiết kế schema, khi cần optimize query,
  khi cần viết migration, khi cần debug database.
version: 2.0.0
---

# SQL Expert

You are a database expert specializing in SQLite (BizClaw's primary DB) and SQL patterns.

## SQLite for BizClaw

### Why SQLite?
- Single file, easy backup
- No server required
- FTS5 for full-text search
- WAL mode for concurrent reads

### Configuration
```rust
// Initialize SQLite with optimizations
let pool = SqlitePoolOptions::new()
    .max_connections(5)
    .acquire_timeout(Duration::from_secs(3))
    .after_connect(|conn| {
        // Enable WAL mode
        conn.execute("PRAGMA journal_mode=WAL", [])?;
        // Foreign keys
        conn.execute("PRAGMA foreign_keys=ON", [])?;
        // Sync mode
        conn.execute("PRAGMA synchronous=NORMAL", [])?;
        // Cache size (negative = KB)
        conn.execute("PRAGMA cache_size=-64000", [])?; // 64MB
        Ok(())
    })
    .connect(&database_url)
    .await?;
```

## Schema Design

### BizClaw Tables
```sql
-- Agents
CREATE TABLE agents (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    model TEXT NOT NULL,
    config TEXT, -- JSON config
    status TEXT NOT NULL DEFAULT 'inactive',
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_agents_status ON agents(status);
CREATE INDEX idx_agents_name ON agents(name);

-- Channels
CREATE TABLE channels (
    id TEXT PRIMARY KEY,
    agent_id TEXT NOT NULL REFERENCES agents(id),
    type TEXT NOT NULL, -- zalo, telegram, facebook
    credentials TEXT NOT NULL, -- encrypted JSON
    status TEXT NOT NULL DEFAULT 'disconnected',
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_channels_agent ON channels(agent_id);
CREATE INDEX idx_channels_type ON channels(type);

-- Messages
CREATE TABLE messages (
    id TEXT PRIMARY KEY,
    agent_id TEXT NOT NULL REFERENCES agents(id),
    channel_id TEXT NOT NULL REFERENCES channels(id),
    direction TEXT NOT NULL, -- inbound, outbound
    content TEXT NOT NULL,
    metadata TEXT, -- JSON
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_messages_agent ON messages(agent_id);
CREATE INDEX idx_messages_created ON messages(created_at);

-- FTS5 for message search
CREATE VIRTUAL TABLE messages_fts USING fts5(
    content,
    content=messages,
    content_rowid=rowid
);
```

## Full-Text Search (FTS5)

### Setup
```sql
-- Create FTS5 table
CREATE VIRTUAL TABLE messages_fts USING fts5(
    content,
    tokenize='unicode61 remove_diacritics 2'
);

-- Triggers to keep FTS in sync
CREATE TRIGGER messages_ai AFTER INSERT ON messages BEGIN
    INSERT INTO messages_fts(rowid, content)
    VALUES (NEW.rowid, NEW.content);
END;

CREATE TRIGGER messages_ad AFTER DELETE ON messages BEGIN
    INSERT INTO messages_fts(messages_fts, rowid, content)
    VALUES ('delete', OLD.rowid, OLD.content);
END;

CREATE TRIGGER messages_au AFTER UPDATE ON messages BEGIN
    INSERT INTO messages_fts(messages_fts, rowid, content)
    VALUES ('delete', OLD.rowid, OLD.content);
    INSERT INTO messages_fts(rowid, content)
    VALUES (NEW.rowid, NEW.content);
END;
```

### Query Patterns
```sql
-- Basic search
SELECT m.* FROM messages m
JOIN messages_fts f ON m.rowid = f.rowid
WHERE messages_fts MATCH 'hello'
ORDER BY rank;

-- Search with filters
SELECT m.* FROM messages m
JOIN messages_fts f ON m.rowid = f.rowid
WHERE messages_fts MATCH 'product AND (price OR cost)'
  AND m.agent_id = ?
ORDER BY rank
LIMIT 50;

-- Snippet preview
SELECT snippet(messages_fts, 0, '<b>', '</b>', '...', 32) as preview
FROM messages_fts
WHERE messages_fts MATCH ?
ORDER BY rank;
```

## Query Optimization

### Indexes
```sql
-- Single column
CREATE INDEX idx_messages_created ON messages(created_at);

-- Composite for common queries
CREATE INDEX idx_messages_agent_created ON messages(agent_id, created_at DESC);

-- Partial index for active records
CREATE INDEX idx_agents_active ON agents(name)
WHERE status = 'active';

-- Covering index (includes all queried columns)
CREATE INDEX idx_messages_covering ON messages(agent_id, created_at DESC)
INCLUDE (content, direction);
```

### Query Patterns
```sql
-- ✅ Good: Use indexed columns first
SELECT * FROM messages
WHERE agent_id = ?  -- indexed
  AND created_at > ?  -- indexed
ORDER BY created_at DESC
LIMIT 50;

-- ❌ Bad: Function on indexed column
SELECT * FROM messages
WHERE date(created_at) = '2024-01-01'  -- can't use index

-- ✅ Good: Date range without function
SELECT * FROM messages
WHERE created_at >= '2024-01-01 00:00:00'
  AND created_at < '2024-01-02 00:00:00'

-- ✅ Good: EXPLAIN QUERY PLAN
EXPLAIN QUERY PLAN
SELECT * FROM messages WHERE agent_id = ?;
```

## Transactions

### Basic Transaction
```rust
// sqlx transaction
let mut tx = pool.begin().await?;

sqlx::query("INSERT INTO agents VALUES (?, ?, ?)")
    .bind(&agent.id)
    .bind(&agent.name)
    .bind(&agent.model)
    .execute(&mut *tx)
    .await?;

sqlx::query("INSERT INTO channels VALUES (?, ?, ?)")
    .bind(&channel.id)
    .bind(&agent.id)
    .bind(&channel.channel_type)
    .execute(&mut *tx)
    .await?;

tx.commit().await?;
```

### Savepoint for Partial Rollback
```sql
SAVEPOINT sp1;

INSERT INTO messages VALUES (...);
INSERT INTO messages_fts(...) VALUES (...);

-- If FTS fails, only rollback FTS
ROLLBACK TO SAVEPOINT sp1;
COMMIT;
```

## Common Patterns

### Pagination
```sql
-- Cursor-based (efficient for large tables)
SELECT * FROM messages
WHERE created_at < ?
ORDER BY created_at DESC
LIMIT 50;

-- Offset-based (slower for large offsets)
SELECT * FROM messages
ORDER BY created_at DESC
LIMIT 50 OFFSET 100;
```

### Aggregation
```sql
-- Count with filters
SELECT
    agent_id,
    COUNT(*) as message_count,
    COUNT(DISTINCT channel_id) as channel_count,
    MIN(created_at) as first_message,
    MAX(created_at) as last_message
FROM messages
WHERE created_at >= date('now', '-7 days')
GROUP BY agent_id;

-- Running total
SELECT
    date(created_at) as day,
    COUNT(*) as daily_count,
    SUM(COUNT(*)) OVER (ORDER BY date(created_at)) as cumulative
FROM messages
GROUP BY date(created_at);
```

### JSON in SQLite
```sql
-- Extract value
SELECT json_extract(config, '$.model') FROM agents;

-- Update JSON field
UPDATE agents
SET config = json_set(config, '$.temperature', 0.7)
WHERE id = ?;

-- Search in JSON
SELECT * FROM agents
WHERE json_extract(config, '$.model') LIKE 'gpt-%';
```

## Rust Examples (sqlx)

```rust
use sqlx::{FromRow, SqlitePool};

#[derive(Debug, FromRow)]
struct Agent {
    id: String,
    name: String,
    model: String,
    status: String,
}

async fn get_active_agents(pool: &SqlitePool) -> Result<Vec<Agent>, sqlx::Error> {
    sqlx::query_as::<_, Agent>(
        "SELECT id, name, model, status FROM agents WHERE status = 'active'"
    )
    .fetch_all(pool)
    .await
}

async fn create_agent(
    pool: &SqlitePool,
    agent: &Agent,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO agents (id, name, model, status) VALUES (?, ?, ?, ?)"
    )
    .bind(&agent.id)
    .bind(&agent.name)
    .bind(&agent.model)
    .bind(&agent.status)
    .execute(pool)
    .await?;
    Ok(())
}
```

## Gotchas

### 1. SQLite Concurrency
```rust
// ❌ Bad: Multiple writers blocked
let pool = SqlitePoolOptions::new()
    .max_connections(100) // Many connections, but only 1 writer

// ✅ Good: Limited connections
let pool = SqlitePoolOptions::new()
    .max_connections(5) // Only need a few for SQLite
```

### 2. FTS Synchronization
```rust
// ❌ Bad: FTS out of sync
INSERT INTO messages ...;
INSERT INTO messages_fts...; // Separate operations = can fail

// ✅ Good: Use triggers
// Triggers handle FTS sync automatically
INSERT INTO messages ...;
-- FTS updated by trigger
```

### 3. Large Transactions
```rust
// ❌ Bad: Large transaction holds locks
sqlx::query("INSERT INTO messages ... VALUES ..."); // millions of rows

// ✅ Good: Batch large inserts
for chunk in messages.chunks(1000) {
    sqlx::query("INSERT INTO messages ...").execute(&mut tx).await?;
}
tx.commit().await?;
```
