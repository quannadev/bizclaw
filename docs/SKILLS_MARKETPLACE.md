# Skills Marketplace

BizClaw's Skills Marketplace provides a registry for discovering, sharing, and installing AI agent skills.

## Overview

The marketplace allows:
- **Discovery**: Find skills by category, tags, or search
- **Installation**: One-click skill installation
- **Publishing**: Share your custom skills
- **Reviews**: Community ratings and feedback

## Marketplace Structure

```
┌─────────────────────────────────────────────────────────────────┐
│                     Skills Marketplace                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐           │
│  │  Skills     │  │  Search     │  │  Categories  │           │
│  │  Registry    │──│  Engine     │──│  & Tags     │           │
│  └─────────────┘  └─────────────┘  └─────────────┘           │
│         │                                                      │
│         ▼                                                      │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐           │
│  │  Version     │  │  Reviews    │  │  Stats &    │           │
│  │  Manager     │  │  System     │  │  Ratings    │           │
│  └─────────────┘  └─────────────┘  └─────────────┘           │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

## Built-in Skills

### Developer

| Skill | Description |
|-------|-------------|
| `web-developer` | Web development assistance, code review, debugging |
| `rust-expert` | Rust best practices, optimization, async patterns |
| `python-analyst` | Python data analysis, ML, automation |
| `sql-expert` | SQL queries, database design, optimization |

### Business

| Skill | Description |
|-------|-------------|
| `vietnamese-business` | Vietnamese business writing, contracts, proposals |
| `content-writer` | Marketing copy, blog posts, social media |
| `devops-engineer` | CI/CD, Docker, Kubernetes, infrastructure |

### Data & Research

| Skill | Description |
|-------|-------------|
| `trend-to-post` | Convert trends into engaging content |
| `research` | Research papers, summarization, citation |

## Skill Structure

A skill consists of:

```
my-skill/
├── SKILL.md          # Main skill definition
├── prompts/          # Additional prompts
├── tools/           # Custom tool definitions
├── config.json       # Skill configuration
└── README.md        # Documentation
```

### SKILL.md Format

```markdown
# My Skill

> A brief description of what this skill does.

## Triggers

- When the user asks about X
- When the user needs Y

## Capabilities

1. Feature A
2. Feature B
3. Feature C

## Usage Examples

### Example 1

User: "..."

Agent: "..."

### Example 2

User: "..."

Agent: "..."

## Limitations

- Cannot do X
- Requires Y

## Configuration

| Setting | Default | Description |
|---------|---------|-------------|
| option1 | value1 | Description |
```

## Marketplace API

### Search Skills

```rust
use bizclaw_skills::marketplace::{Marketplace, SkillSearchQuery, SkillCategory};

let mut marketplace = Marketplace::new();

// Search by query
let results = marketplace.search(SkillSearchQuery {
    query: Some("python".to_string()),
    ..Default::default()
});

// Search by category
let results = marketplace.search(SkillSearchQuery {
    category: Some(SkillCategory::Developer),
    ..Default::default()
});

// Combined search with filters
let results = marketplace.search(SkillSearchQuery {
    query: Some("web".to_string()),
    category: Some(SkillCategory::Developer),
    min_rating: Some(4.0),
    sort_by: SortOption::Rating,
    ..Default::default()
});
```

### Get Skill Details

```rust
if let Some(skill) = marketplace.get("web-developer") {
    println!("Name: {}", skill.name);
    println!("Author: {}", skill.author.name);
    println!("Downloads: {}", skill.metadata.downloads);
    println!("Rating: {:.1}", skill.metadata.rating);
}
```

### List Popular/Recent

```rust
// Most popular skills
let popular = marketplace.list_popular(10);

// Recently updated
let recent = marketplace.list_recent(10);
```

### Categories

```rust
let developer_skills = marketplace.list_by_category(&SkillCategory::Developer);
let python_skills = marketplace.list_by_tag("python");
```

## CLI Usage

### List Skills

```bash
# List all available skills
bizclaw skills list

# List skills by category
bizclaw skills list --category developer

# Search skills
bizclaw skills search "web development"
```

### Install Skill

```bash
# Install a skill
bizclaw skills install web-developer

# Install specific version
bizclaw skills install web-developer --version 1.2.0

# Install from URL
bizclaw skills install https://registry.bizclaw.ai/skills/custom-skill
```

### Manage Skills

```bash
# View skill info
bizclaw skills info web-developer

# Update skill
bizclaw skills update web-developer

# Uninstall skill
bizclaw skills uninstall web-developer

# List installed skills
bizclaw skills list --installed
```

## Publishing Skills

### Create Skill

```bash
# Initialize a new skill
bizclaw skills create my-skill --template developer
```

### Publish Skill

```bash
# Package and publish
bizclaw skills publish --dry-run  # Preview
bizclaw skills publish           # Publish to registry
```

### Skill Manifest

Create `skill.json`:

```json
{
  "id": "my-custom-skill",
  "name": "My Custom Skill",
  "version": "1.0.0",
  "description": "What this skill does",
  "author": {
    "name": "Your Name",
    "email": "you@example.com"
  },
  "category": "developer",
  "tags": ["python", "automation"],
  "minBizClawVersion": "1.1.0",
  "license": "MIT",
  "files": [
    "SKILL.md",
    "prompts/*.md"
  ]
}
```

## Categories

| Category | Description |
|----------|-------------|
| `developer` | Programming and development skills |
| `business` | Business writing and communication |
| `creative` | Content creation and design |
| `data` | Data analysis and processing |
| `automation` | Workflow and process automation |
| `communication` | Communication and collaboration |
| `research` | Research and analysis |
| `education` | Teaching and learning |
| `other` | Miscellaneous skills |

## Reviews & Ratings

### Add Review

```rust
let review = Review {
    id: "review-123".to_string(),
    user_id: "user-456".to_string(),
    user_name: "John Doe".to_string(),
    rating: 5,
    comment: "Excellent skill, very helpful!".to_string(),
    created_at: Utc::now(),
};

marketplace.add_review("web-developer", review)?;
```

### View Reviews

```rust
if let Some(skill) = marketplace.get("web-developer") {
    for review in &skill.reviews {
        println!("{}: {} stars - {}", review.user_name, review.rating, review.comment);
    }
}
```

## Stats

| Metric | Description |
|--------|-------------|
| `downloads` | Total times skill was downloaded |
| `installs` | Total installations |
| `active_users` | Users with skill currently active |
| `avg_response_time_ms` | Average execution time |
| `success_rate` | Percentage of successful executions |

## Versioning

### Semantic Versioning

Skills follow semver:
- **Major**: Breaking changes
- **Minor**: New features
- **Patch**: Bug fixes

### Version Commands

```bash
# Check for updates
bizclaw skills check-updates web-developer

# Update to latest
bizclaw skills update web-developer

# Pin to version
bizclaw skills install web-developer --version 1.2.0
```

## Security

### Verification

Verified authors have a badge:

```
✅ Verified Author
```

### Sandboxing

Skills run in isolated environments:
- Limited file system access
- No network access (by default)
- Timeout limits
- Resource constraints

### Permissions

| Permission | Description |
|------------|-------------|
| `file:read` | Read files |
| `file:write` | Write files |
| `network` | Make HTTP requests |
| `exec` | Run commands |

## Troubleshooting

### Skill not installing

1. Check network connectivity
2. Verify BizClaw version meets requirements
3. Check disk space

### Skill not working

1. Review skill documentation
2. Check configuration settings
3. Update to latest version

### Publishing fails

1. Verify your account is verified
2. Check skill format is correct
3. Review error message

## Examples

See the test module in `bizclaw-skills/src/marketplace.rs` for complete examples.

## Future Features

- Skill dependencies
- Skill bundles/discounts
- Skill subscriptions
- Private skill registry
- Skill analytics dashboard
