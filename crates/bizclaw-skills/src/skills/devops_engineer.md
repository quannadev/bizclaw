---
name: devops-engineer
description: |
  DevOps engineer for BizClaw deployment, CI/CD, Docker, and infrastructure. Trigger phrases:
  deploy, CI/CD, Docker, Kubernetes, GitHub Actions, production, staging, nginx, SSL,
  reverse proxy, load balancing, monitoring, deployment, infrastructure, server setup.
  Scenarios: khi cần deploy production, khi cần setup CI/CD, khi cần configure Docker,
  khi cần setup nginx, khi cần monitoring.
version: 2.0.0
---

# DevOps Engineer

You are a DevOps engineer specializing in CI/CD, containerization, and infrastructure for BizClaw.

## Docker

### Multi-Stage Build
```dockerfile
# Stage 1: Build
FROM rust:1.85-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN cargo build --release --bin bizclaw

# Stage 2: Runtime
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/bizclaw /usr/local/bin/
EXPOSE 8080
CMD ["bizclaw", "serve"]
```

### Security Best Practices
```dockerfile
# Use specific version, not 'latest'
FROM rust:1.85-bookworm

# Create non-root user
RUN groupadd -r bizclaw && useradd -r -g bizclaw bizclaw
USER bizclaw

# Read-only filesystem
VOLUME ["/data"]

# No secrets in image
ARG BUILD_ARG
ENV BUILD_ARG=${BUILD_ARG}
```

## GitHub Actions

### CI Pipeline
```yaml
name: CI

on:
  push:
    branches: [main, master]
  pull_request:
    branches: [main, master]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6

      - name: Install Rust
        uses: dtolnay/rust-action@stable
        with:
          components: clippy, rustfmt

      - name: Cache cargo
        uses: Swatinem/rust-cache@v2

      - name: Format check
        run: cargo fmt --all -- --check

      - name: Clippy
        run: cargo clippy --all-targets -- -D warnings

      - name: Test
        run: cargo test --workspace --lib

      - name: Security audit
        run: cargo audit
```

### Docker Build & Push
```yaml
name: Docker

on:
  push:
    tags: ['v*']

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6

      - uses: docker/setup-qemu-action@v4
      - uses: docker/setup-buildx-action@v3

      - uses: docker/metadata-action@v6
        id: meta
        with:
          images: ghcr.io/${{ github.repository }}
          tags: type=semver,pattern={{version}}

      - uses: docker/build-push-action@v7
        with:
          context: .
          push: ${{ github.event_name != 'pull_request' }}
          tags: ${{ steps.meta.outputs.tags }}
          labels: ${{ steps.meta.outputs.labels }}
          cache-from: type=gha
          cache-to: type=gha,mode=max
```

## Infrastructure

### Nginx Reverse Proxy
```nginx
server {
    listen 443 ssl http2;
    server_name bizclaw.vn;

    ssl_certificate /etc/letsencrypt/live/bizclaw.vn/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/bizclaw.vn/privkey.pem;

    # Security headers
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header X-XSS-Protection "1; mode=block" always;
    add_header Strict-Transport-Security "max-age=31536000" always;

    # Rate limiting
    limit_req_zone $binary_remote_addr zone=api:10m rate=10r/s;

    location / {
        proxy_pass http://127.0.0.1:3001;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_cache_bypass $http_upgrade;

        limit_req zone=api burst=20 nodelay;
    }

    location /api/ {
        proxy_pass http://127.0.0.1:8080/api/;
        limit_req zone=api burst=50 nodelay;
    }
}
```

### Docker Compose Production
```yaml
version: '3.8'

services:
  bizclaw:
    image: ghcr.io/nguyenduchoai/bizclaw:latest
    restart: unless-stopped
    ports:
      - "127.0.0.1:8080:8080"
    volumes:
      - bizclaw-data:/data
      - ./config.toml:/app/config.toml:ro
    environment:
      - DATABASE_URL=sqlite:/data/bizclaw.db
      - RUST_LOG=info
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3

  nginx:
    image: nginx:alpine
    restart: unless-stopped
    ports:
      - "443:443"
      - "80:80"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf:ro
      - ./ssl:/etc/letsencrypt:ro
    depends_on:
      - bizclaw

volumes:
  bizclaw-data:
    driver: local
```

## Monitoring

### Prometheus Metrics
```yaml
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'bizclaw'
    static_configs:
      - targets: ['127.0.0.1:8080']
    metrics_path: /metrics
```

### Health Check
```rust
use axum::{extract::State, http::StatusCode, Json};
use serde::Serialize;

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    version: String,
    uptime_seconds: u64,
}

async fn health_check(State(state): State<AppState>) -> Result<Json<HealthResponse>, StatusCode> {
    Ok(Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: state.start_time.elapsed().as_secs(),
    }))
}
```

## Deployment Checklist

### Pre-Deployment
- [ ] Tests pass locally
- [ ] cargo clippy clean
- [ ] cargo audit passed
- [ ] Secrets rotated
- [ ] Backup created

### Post-Deployment
- [ ] Health check passes
- [ ] Smoke test passed
- [ ] Logs clean
- [ ] Metrics normal
- [ ] Rollback plan ready

### Rollback
```bash
#!/bin/bash
# Quick rollback to previous version

# Stop current
docker stop bizclaw

# Restore previous image tag
docker pull ghcr.io/nguyenduchoai/bizclaw:previous

# Start with previous
docker run -d --name bizclaw \
    -p 127.0.0.1:8080:8080 \
    -v bizclaw-data:/data \
    ghcr.io/nguyenduchoai/bizclaw:previous
```

## Validation

```bash
#!/bin/bash
echo "=== Pre-Deploy Validation ==="

# Check Docker build
docker build -t bizclaw:test . || exit 1

# Run container health check
docker run -d --name bizclaw-test bizclaw:test
sleep 5
curl -f http://localhost:8080/health || { docker rm bizclaw-test; exit 1; }
docker rm bizclaw-test

# Security scan
docker scout cves bizclaw:test || echo "⚠️ CVEs found"

echo "✅ Validation passed"
```
