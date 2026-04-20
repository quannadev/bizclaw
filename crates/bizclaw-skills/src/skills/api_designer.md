---
name: api-designer
description: |
  API designer for BizClaw REST and WebSocket APIs. Trigger phrases: design API,
  create endpoint, REST API, WebSocket, API design, endpoint specification,
  thiết kế API, tạo endpoint, REST API, WebSocket, giao thức.
  Scenarios: khi cần thiết kế API mới, khi cần thêm endpoint, khi cần protocol design,
  khi cần xem xét API specification.
version: 2.0.0
---

# API Designer

You are an API designer specializing in REST and WebSocket protocols for BizClaw.

## REST API Design

### URL Structure
```
/api/v1/{resource}
/api/v1/{resource}/{id}
/api/v1/{resource}/{id}/{sub-resource}
```

### HTTP Methods
| Method | Purpose | Idempotent | Safe |
|--------|---------|------------|------|
| GET | Read resource | Yes | Yes |
| POST | Create resource | No | No |
| PUT | Replace resource | Yes | No |
| PATCH | Update partial | No | No |
| DELETE | Delete resource | Yes | No |

### Status Codes
```markdown
# Success
200 OK - Successful GET/PATCH
201 Created - Successful POST
204 No Content - Successful DELETE

# Client Error
400 Bad Request - Invalid input
401 Unauthorized - Missing auth
403 Forbidden - Insufficient permission
404 Not Found - Resource doesn't exist
409 Conflict - Duplicate resource
422 Unprocessable Entity - Validation error

# Server Error
500 Internal Server Error
503 Service Unavailable
```

### BizClaw API Conventions

```rust
// Request/Response pattern
#[derive(Deserialize)]
struct CreateAgentRequest {
    name: String,
    model: String,
    #[serde(default)]
    description: Option<String>,
}

#[derive(Serialize)]
struct AgentResponse {
    id: String,
    name: String,
    model: String,
    status: String,
    created_at: DateTime<Utc>,
}

#[derive(Serialize)]
struct ListResponse<T> {
    items: Vec<T>,
    total: usize,
    page: usize,
    per_page: usize,
}

// Error response
#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<Value>,
}
```

### REST Endpoints

```rust
// Agents
GET    /api/v1/agents                    - List agents
POST   /api/v1/agents                    - Create agent
GET    /api/v1/agents/:id               - Get agent
PATCH  /api/v1/agents/:id               - Update agent
DELETE /api/v1/agents/:id               - Delete agent

// Channels
GET    /api/v1/channels                  - List channels
POST   /api/v1/channels/zalo            - Connect Zalo
POST   /api/v1/channels/telegram        - Connect Telegram
POST   /api/v1/channels/facebook        - Connect Facebook
DELETE /api/v1/channels/:id             - Disconnect channel

// Messages
GET    /api/v1/messages                 - List messages
POST   /api/v1/messages                 - Send message
GET    /api/v1/messages/:id             - Get message

// Campaigns
GET    /api/v1/campaigns                - List campaigns
POST   /api/v1/campaigns                - Create campaign
GET    /api/v1/campaigns/:id            - Get campaign
PATCH  /api/v1/campaigns/:id            - Update campaign
POST   /api/v1/campaigns/:id/execute   - Execute campaign
```

## WebSocket Protocol

### Connection
```javascript
// Client connects to:
ws://host:8080/ws?token=JWT_TOKEN

// Server accepts with:
{ "type": "connected", "session_id": "uuid" }
```

### Message Format
```json
{
  "type": "message_type",
  "payload": {},
  "timestamp": "2024-01-01T00:00:00Z",
  "id": "message_uuid"
}
```

### Message Types

#### Client → Server
```json
// Send message
{
  "type": "send_message",
  "payload": {
    "channel": "zalo",
    "recipient": "user_id",
    "content": "Hello"
  }
}

// Get agent status
{
  "type": "get_agent_status",
  "payload": { "agent_id": "uuid" }
}
```

#### Server → Client
```json
// Message received
{
  "type": "message_received",
  "payload": {
    "channel": "zalo",
    "sender": "user_id",
    "content": "Hello",
    "agent_response": "Hi there!"
  }
}

// Error
{
  "type": "error",
  "payload": {
    "code": "RATE_LIMITED",
    "message": "Too many requests"
  }
}
```

### Rate Limiting
```rust
// Per-connection rate limiting
struct RateLimiter {
    tokens: usize,
    max_tokens: usize,
    refill_rate: Duration,
}

impl RateLimiter {
    fn try_acquire(&mut self) -> bool {
        if self.tokens > 0 {
            self.tokens -= 1;
            true
        } else {
            false
        }
    }
}
```

## Validation

### Input Validation
```rust
use validator::Validate;

#[derive(Deserialize, Validate)]
struct CreateAgentRequest {
    #[validate(length(min = 1, max = 100))]
    name: String,

    #[validate(email)]
    #[serde(default)]
    email: Option<String>,

    #[validate(range(min = 0, max = 1000000))]
    #[serde(default)]
    max_tokens: Option<u32>,
}

fn create_agent(Json(req): Json<CreateAgentRequest>) -> Result<(), AppError> {
    req.validate().map_err(AppError::Validation)?;
    // ...
}
```

### Schema Documentation
```markdown
# POST /api/v1/agents

Create a new AI agent.

## Request Body
```json
{
  "name": "string (required, 1-100 chars)",
  "model": "string (required, e.g. 'gpt-4')",
  "description": "string (optional)"
}
```

## Response 201
```json
{
  "id": "uuid",
  "name": "Agent Name",
  "model": "gpt-4",
  "status": "active",
  "created_at": "2024-01-01T00:00:00Z"
}
```

## Errors
- 400: Invalid input
- 401: Unauthorized
- 409: Agent name already exists
```

## Gotchas

### 1. Pagination
```rust
// ❌ Bad: No pagination
GET /api/v1/messages → returns ALL messages

// ✅ Good: Cursor-based pagination
GET /api/v1/messages?cursor=xxx&limit=50 → returns 50 messages

// Response includes next cursor
{
  "items": [...],
  "next_cursor": "encoded_cursor",
  "has_more": true
}
```

### 2. Idempotency
```rust
// Use idempotency key for POST requests
POST /api/v1/messages
Headers: Idempotency-Key: unique-key

// If key was used before, return cached response
```

### 3. Error Codes
```rust
// ❌ Bad: Generic error
{ "error": "Something went wrong" }

// ✅ Good: Specific error code
{
  "error": "VALIDATION_ERROR",
  "message": "Invalid request body",
  "details": {
    "field": "email",
    "reason": "Invalid email format"
  }
}
```
