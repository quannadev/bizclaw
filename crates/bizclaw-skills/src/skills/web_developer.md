---
name: web-developer
description: |
  Full-stack web developer for BizClaw frontend and API development. Trigger phrases:
  add this page, create component, fix UI, improve UX, responsive design, React component,
  API endpoint, frontend, backend, dashboard page, landing page, fix CSS, add this feature.
  Scenarios: khi cần tạo trang mới, khi cần fix UI/UX, khi cần thêm API endpoint,
  khi cần cải thiện performance, khi cần responsive design.
version: 2.0.0
---

# Web Developer

You are a full-stack web developer for the BizClaw platform.

## Frontend Stack

### Core Technologies
- **JavaScript/TypeScript**: ES2022+, strict mode, module system
- **React 18+**: Hooks, Suspense, Server Components
- **CSS**: Flexbox, Grid, CSS Variables, Custom Properties

### BizClaw Frontend Patterns
```javascript
// ✅ Component pattern
export function DashboardPage({ agentId }) {
  const [data, setData] = useState(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    fetchAgentData(agentId)
      .then(setData)
      .finally(() => setLoading(false));
  }, [agentId]);

  if (loading) return <LoadingSpinner />;
  if (!data) return <EmptyState />;

  return (
    <div className="dashboard">
      <AgentHeader agent={data.agent} />
      <MetricsGrid metrics={data.metrics} />
    </div>
  );
}
```

### State Management
```javascript
// ✅ Use Zustand for global state
import { create } from 'zustand';

const useAgentStore = create((set) => ({
  agents: [],
  selectedAgent: null,
  setAgents: (agents) => set({ agents }),
  selectAgent: (id) => set({ selectedAgent: id }),
}));

// ✅ Use React Query for server state
import { useQuery, useMutation } from '@tanstack/react-query';

function useAgents() {
  return useQuery({
    queryKey: ['agents'],
    queryFn: fetchAgents,
    staleTime: 5 * 60 * 1000, // 5 minutes
  });
}
```

### CSS Architecture
```css
/* Use CSS Variables for theming */
:root {
  --color-primary: #10b981;
  --color-secondary: #3b82f6;
  --spacing-md: 1rem;
  --radius-lg: 1rem;
}

/* Mobile-first responsive */
.card {
  padding: var(--spacing-md);
}

@media (min-width: 768px) {
  .card {
    padding: calc(var(--spacing-md) * 2);
  }
}
```

## Backend (Axum)

### REST API Pattern
```rust
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct CreateAgentRequest {
    name: String,
    model: String,
}

#[derive(Serialize)]
struct AgentResponse {
    id: String,
    name: String,
    status: String,
}

async fn create_agent(
    State(db): State<Database>,
    Json(payload): Json<CreateAgentRequest>,
) -> Result<Json<AgentResponse>, StatusCode> {
    let agent = db.create_agent(payload.name, payload.model)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(AgentResponse {
        id: agent.id,
        name: agent.name,
        status: "active".to_string(),
    }))
}
```

### WebSocket Handler
```rust
use axum::{
    extract::ws::{WebSocket, Message, WebSocketUpgrade},
    response::Response,
};

async fn ws_handler(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket))
}

async fn handle_socket(mut socket: WebSocket) {
    while let Some(msg) = socket.recv().await {
        if let Ok(msg) = msg {
            // Process message with SecretRedactor
            let redacted = SecretRedactor::new().redact(msg.to_string().as_str());
            handle_message(redacted).await;
        }
    }
}
```

## Quality Standards

### Performance
- **Core Web Vitals**: LCP < 2.5s, FID < 100ms, CLS < 0.1
- **Bundle size**: Code split, lazy load routes
- **Images**: WebP, lazy loading, srcset for responsive

### Accessibility (WCAG 2.1 AA)
- Semantic HTML elements
- ARIA labels for interactive elements
- Keyboard navigation support
- Color contrast ratios

### Security
- **XSS Prevention**: Sanitize user input, CSP headers
- **CSRF**: Token validation on state-changing requests
- **CORS**: Whitelist origins, not `*` for credentials

## BizClaw-Specific

### Dashboard Pages
The BizClaw dashboard uses a modular page system:
```javascript
// In app.js - register new pages
registerPage({
  path: '/campaigns',
  title: 'Campaigns',
  icon: '📢',
  component: CampaignPage,
  menu: ['dashboard', 'campaigns']
});
```

### API Endpoints
All API routes follow REST conventions:
```
GET    /api/v1/agents        - List agents
POST   /api/v1/agents        - Create agent
GET    /api/v1/agents/:id    - Get agent
PATCH  /api/v1/agents/:id    - Update agent
DELETE /api/v1/agents/:id    - Delete agent

GET    /api/v1/channels      - List channels
POST   /api/v1/channels/zalo - Connect Zalo
POST   /api/v1/channels/telegram - Connect Telegram
```

## Gotchas

### 1. Axum State Cloning
```rust
// ❌ Bad: Clone inside handler
async fn handler(State(state): State<Arc<Db>>) {
    let db = state.clone(); // Unnecessary clone
}

// ✅ Good: Clone only when needed
async fn handler(State(state): State<Arc<Db>>) {
    let db = state.as_ref(); // Reference only
}
```

### 2. CORS Configuration
```rust
// Always specify allowed origins in production
let cors = CorsLayer::new()
    .allow_origin("https://bizclaw.vn".parse::<HeaderValue>().unwrap())
    .allow_methods(GET | POST | PATCH | DELETE)
    .allow_credentials(true);
```

### 3. React StrictMode Double Mount
```javascript
// useEffect runs twice in development - handle gracefully
useEffect(() => {
  let mounted = true;
  fetchData().then(data => {
    if (mounted) setData(data);
  });
  return () => { mounted = false; };
}, []);
```
