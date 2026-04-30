# BizClaw Marketplace

The official web marketplace for BizClaw AI skills.

## Features

- 🔍 **Skill Browser** - Search and discover skills by category
- 📁 **Categories** - Browse skills by category (Developer, Business, Data, etc.)
- ⭐ **Ratings & Reviews** - Community ratings and detailed reviews
- 📥 **One-click Install** - Install skills directly to your BizClaw
- 🔥 **Trending** - See what's popular in the community

## Development

```bash
# Start the gateway
cd ../bizclaw
cargo run --bin bizclaw -- gateway

# Open marketplace in browser
# Visit http://localhost:18789/marketplace
```

## API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/marketplace/stats` | GET | Get marketplace statistics |
| `/api/marketplace/categories` | GET | List all categories |
| `/api/marketplace/featured` | GET | Get featured/trending skills |
| `/api/marketplace/skills` | GET | Search skills |
| `/api/marketplace/skills/:slug` | GET | Get skill details |
| `/api/marketplace/skills/:slug/install` | POST | Install a skill |
| `/api/marketplace/skills/:slug/reviews` | GET/POST | Get/create reviews |

## Categories

| Category | Icon | Description |
|----------|------|-------------|
| Developer | 💻 | Programming and development skills |
| Business | 💼 | Business writing and communication |
| Creative | 🎨 | Content creation and design |
| Data | 📊 | Data analysis and processing |
| Automation | ⚡ | Workflow and process automation |
| Communication | 💬 | Communication and collaboration |
| Research | 🔬 | Research and analysis |
| Education | 📚 | Teaching and learning |

## Built-in Skills

| Skill | Category | Description |
|-------|----------|-------------|
| Web Developer | Developer | HTML, CSS, JS, React, Vue assistance |
| Python Analyst | Data | Python data analysis and ML |
| Rust Expert | Developer | Rust best practices |
| Vietnamese Business | Business | Vietnamese business writing |
| Content Writer | Creative | Marketing copy and blogs |
| DevOps Engineer | Developer | CI/CD and infrastructure |

## Screenshots

Coming soon...

## License

MIT
