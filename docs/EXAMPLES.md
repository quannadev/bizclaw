# 📚 BizClaw Examples

## 🤖 Agent Examples

### Example 1: Basic Agent
```rust
use bizclaw_agent::{Agent, AgentConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let agent = Agent::new(AgentConfig::default())
        .await?;
    
    let response = agent.process("Xin chào").await?;
    println!("Response: {}", response);
    
    Ok(())
}
```

### Example 2: Agent with Tools
```rust
use bizclaw_agent::{Agent, AgentConfig};
use bizclaw_tools::{ToolRegistry, BrowserTool, FileTool};

let mut tools = ToolRegistry::new();
tools.register(BrowserTool::new());
tools.register(FileTool::new());

let agent = Agent::builder()
    .config(AgentConfig::default())
    .tools(tools)
    .build()
    .await?;

let response = agent.process("Tìm kiếm thông tin trên web").await?;
```

### Example 3: Multi-Agent Team
```rust
use bizclaw_orchestrator::{Orchestrator, AgentTeam};

let team = AgentTeam::builder()
    .add("collector", CollectorAgent::new())
    .add("writer", WriterAgent::new())
    .add("publisher", PublisherAgent::new())
    .build();

let orchestrator = Orchestrator::new(team);
orchestrator.run("Tạo bài viết về sản phẩm mới").await?;
```

---

## 💬 Channel Examples

### Zalo Channel
```rust
use bizclaw_channels::{ZaloChannel, ZaloConfig};

let zalo = ZaloChannel::new(ZaloConfig {
    app_id: "your_app_id".into(),
    app_secret: "your_app_secret".into(),
}).await?;

zalo.on_message(|msg| async move {
    println!("Received: {}", msg.content);
    Ok("Xin chào! Tôi có thể giúp gì cho bạn?".into())
}).await?;
```

### Telegram Bot
```rust
use bizclaw_channels::{TelegramBot, TelegramConfig};

let bot = TelegramBot::new(TelegramConfig {
    token: "your_bot_token".into(),
}).await?;

bot.on_command("/start", |ctx| async move {
    ctx.reply("Chào bạn! 👋").await
}).await?;

bot.on_message(|msg| async move {
    println!("Message: {}", msg.text);
}).await?;
```

### Discord Bot
```rust
use bizclaw_channels::{DiscordBot, DiscordConfig};

let bot = DiscordBot::new(DiscordConfig {
    token: "your_discord_token".into(),
    guild_id: "your_guild_id".into(),
}).await?;

bot.on_message(|msg| async move {
    if msg.content.starts_with("!ping") {
        msg.reply("Pong! 🏓").await?;
    }
}).await?;
```

---

## 🧠 Memory Examples

### Store and Search
```rust
use bizclaw_memory::{Memory, MemoryConfig};

let memory = Memory::new(MemoryConfig::default()).await?;

memory.store("user_order", "Customer ordered 2 shirts").await?;
memory.store("product_info", "Blue shirt, size M, $25").await?;

// Semantic search
let results = memory.search("customer purchase").await?;
println!("Found: {:?}", results);
```

### Vector Search
```rust
use bizclaw_memory::VectorStore;

let vectors = VectorStore::new().await?;
vectors.insert("doc1", vec![0.1, 0.2, 0.3]).await?;
vectors.insert("doc2", vec![0.4, 0.5, 0.6]).await?;

let results = vectors.similarity_search(vec![0.1, 0.2, 0.3], 2).await?;
```

---

## 🛠️ Tool Examples

### Browser Automation
```rust
use bizclaw_tools::{Browser, BrowserConfig};

let browser = Browser::new(BrowserConfig::default()).await?;
let page = browser.new_page("https://example.com").await?;

page.goto("https://example.com/products").await?;
page.click(".buy-button").await?;
page.fill(".quantity", "2").await?;
page.click(".checkout").await?;

let screenshot = page.screenshot().await?;
```

### Database Query
```rust
use bizclaw_tools::{Database, DbConfig};

let db = Database::new(DbConfig {
    url: "sqlite:bizclaw.db".into(),
}).await?;

let results = db.query("SELECT * FROM customers WHERE active = 1").await?;
for row in results {
    println!("Customer: {}", row.get("name"));
}
```

### HTTP Request
```rust
use bizclaw_tools::{HttpClient, HttpRequest};

let client = HttpClient::new();
let response = client.get("https://api.example.com/users").await?;

println!("Status: {}", response.status);
println!("Body: {}", response.body);
```

---

## 📝 Content Generation

### Generate Marketing Content
```rust
use bizclaw_content::{ContentGenerator, ContentConfig};

let generator = ContentConfig::builder()
    .provider("openai")
    .model("gpt-4o")
    .style("friendly, professional")
    .build();

let content = generator.generate("Tạo bài viết bán áo thun", 
    &["giảm giá", "mùa hè", "thoải mái"]
).await?;

println!("Title: {}", content.title);
println!("Body: {}", content.body);
```

### Schedule Content
```rust
use bizclaw_content::{Scheduler, ScheduleConfig};

let scheduler = Scheduler::new(ScheduleConfig::default()).await?;

scheduler.schedule("daily_promo", |ctx| async move {
    let content = ctx.generate_content("Khuyến mãi hàng ngày").await?;
    ctx.post_to_all_channels(content).await
}).cron("0 9 * * *").await?; // 9 AM daily
```

---

## 🔐 Security Examples

### API Key Management
```rust
use bizclaw_security::{Vault, VaultConfig};

let vault = Vault::new(VaultConfig {
    master_key: "your_master_key".into(),
}).await?;

vault.store("openai_key", "sk-xxx").await?;
vault.store("zalo_secret", "xxx").await?;

let api_key = vault.get("openai_key").await?;
```

### Input Validation
```rust
use bizclaw_security::{Validator, ValidationRules};

let validator = ValidationRules::new()
    .max_length(1000)
    .no_sql_injection()
    .no_xss()
    .build();

validator.validate("user_input").await?;
```

---

## ⚙️ Configuration Examples

### Complete Config
```toml
# config.toml
[app]
name = "BizClaw"
version = "1.1.7"

[server]
host = "0.0.0.0"
port = 8080

[channels.telegram]
enabled = true
bot_token = "YOUR_TOKEN"

[channels.zalo]
enabled = true
app_id = "YOUR_APP_ID"
app_secret = "YOUR_SECRET"

[ai]
provider = "openai"
model = "gpt-4o"

[memory]
vector_dimensions = 1536
max_entries = 10000
```

### Environment Variables
```bash
export BIZCLAW_PORT=8080
export BIZCLAW_HOST=0.0.0.0
export OPENAI_API_KEY=sk-xxx
export TELEGRAM_BOT_TOKEN=xxx
export ZALO_APP_ID=xxx
export ZALO_APP_SECRET=xxx
```

---

## 🚀 Deployment Examples

### Docker
```dockerfile
FROM ghcr.io/nguyenduchoai/bizclaw:latest
ENV PORT=8080
EXPOSE 8080
CMD ["bizclaw", "serve"]
```

### Docker Compose
```yaml
version: '3.8'
services:
  bizclaw:
    image: ghcr.io/nguyenduchoai/bizclaw:latest
    ports:
      - "8080:8080"
    environment:
      - DATABASE_URL=sqlite:/data/bizclaw.db
      - OPENAI_API_KEY=${OPENAI_API_KEY}
    volumes:
      - ./data:/data

  postgres:
    image: postgres:16
    environment:
      - POSTGRES_DB=bizclaw
      - POSTGRES_USER=bizclaw
      - POSTGRES_PASSWORD=password
    volumes:
      - pgdata:/var/lib/postgresql/data

volumes:
  pgdata:
```

---

## 🧪 Testing Examples

### Unit Test
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_generation() {
        let content = generate_sample_content();
        assert!(!content.is_empty());
        assert!(content.contains("title"));
    }

    #[test]
    fn test_validation() {
        let validator = ValidationRules::new().build();
        assert!(validator.validate("safe input").is_ok());
        assert!(validator.validate("DROP TABLE").is_err());
    }
}
```

### Integration Test
```rust
#[tokio::test]
async fn test_agent_with_memory() {
    let agent = Agent::new(AgentConfig::default()).await.unwrap();
    let memory = Memory::new(MemoryConfig::default()).await.unwrap();
    
    memory.store("test", "test value").await.unwrap();
    
    let response = agent.process("What is the test value?").await.unwrap();
    assert!(response.contains("test value"));
}
```
