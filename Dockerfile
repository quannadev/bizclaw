# ═══════════════════════════════════════════════════════════════
# BizClaw AI Agent Platform — Optimized Multi-stage Docker Build
# Self-hosted on Pi, VPS, or any Linux machine
# ═══════════════════════════════════════════════════════════════

# Stage 1: Build
FROM rust:latest AS builder

WORKDIR /build

# Copy workspace Cargo files first (dependency caching layer)
COPY Cargo.toml Cargo.lock ./

# Copy crate manifests only (for dep resolution)
COPY crates/ crates/
COPY src/ src/
COPY data/ data/
COPY migrations/ migrations/

# Build release binaries with optimizations
RUN cargo build --release --bin bizclaw --bin bizclaw-platform

# Stage 2: Runtime — minimal image with only what's needed
FROM debian:trixie-slim AS runtime

# Install minimal runtime deps (docker-cli only, not full docker.io) + yt-dlp/ffmpeg for media extraction
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 curl python3 python3-pip yt-dlp ffmpeg \
    && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*

# Install docker CLI only (much smaller than docker.io)
COPY --from=docker:27-cli /usr/local/bin/docker /usr/local/bin/docker

# Copy binaries from builder
COPY --from=builder /build/target/release/bizclaw /usr/local/bin/bizclaw
COPY --from=builder /build/target/release/bizclaw-platform /usr/local/bin/bizclaw-platform

# ── Security: Non-root user ──────────────────────────────────
# Create dedicated user to run the application (no root access)
RUN groupadd -r bizclaw && useradd -r -g bizclaw -m -s /bin/false bizclaw

# Create data directory with proper ownership
RUN mkdir -p /home/bizclaw/.bizclaw && chown -R bizclaw:bizclaw /home/bizclaw/.bizclaw

# Environment — GMT+7
ENV BIZCLAW_CONFIG=/home/bizclaw/.bizclaw/config.toml
ENV BIZCLAW_DATA_DIR=/home/bizclaw/.bizclaw
ENV HOME=/home/bizclaw
ENV RUST_LOG=info
ENV TZ=Asia/Ho_Chi_Minh

# Expose ports: platform admin (3001) + tenant gateways (10001-10010)
EXPOSE 3001 10001 10002 10003 10004 10005 10006 10007 10008 10009 10010

# Health check
HEALTHCHECK --interval=30s --timeout=5s --retries=3 \
    CMD curl -f http://localhost:3001/health || exit 1

# OCI Labels
LABEL org.opencontainers.image.title="BizClaw AI Platform"
LABEL org.opencontainers.image.description="Multi-tenant AI Agent platform for SME businesses"
LABEL org.opencontainers.image.version="0.3.2"

# ── Switch to non-root user ──────────────────────────────────
USER bizclaw

# Default: run the platform
ENTRYPOINT ["bizclaw-platform"]
CMD ["--port", "3001", "--bizclaw-bin", "/usr/local/bin/bizclaw"]

