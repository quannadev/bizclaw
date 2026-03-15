#!/usr/bin/env bash
# ════════════════════════════════════════════════════════════════
# BizClaw 1-Click Install Script
#
# Usage:
#   curl -sSL https://bizclaw.vn/install.sh | sudo bash -s -- \
#     --domain bot.company.vn \
#     --admin-email admin@company.vn
#
# What it does:
#   1. Installs PostgreSQL 16
#   2. Creates bizclaw database + user
#   3. Downloads BizClaw binary (or builds from source)
#   4. Creates config.toml
#   5. Sets up Nginx reverse proxy
#   6. Installs SSL certificate (Let's Encrypt)
#   7. Creates systemd service (auto-restart)
#
# Supports: Ubuntu 22.04+, Debian 12+
# ════════════════════════════════════════════════════════════════

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log() { echo -e "${GREEN}[BizClaw]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARNING]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; exit 1; }

# ── Parse arguments ──────────────────────────────────────────

DOMAIN=""
ADMIN_EMAIL=""
DB_PASSWORD=""
BIZCLAW_PORT=3001
INSTALL_DIR="/opt/bizclaw"
BIZCLAW_USER="bizclaw"

while [[ $# -gt 0 ]]; do
    case $1 in
        --domain) DOMAIN="$2"; shift 2 ;;
        --admin-email) ADMIN_EMAIL="$2"; shift 2 ;;
        --db-password) DB_PASSWORD="$2"; shift 2 ;;
        --port) BIZCLAW_PORT="$2"; shift 2 ;;
        --install-dir) INSTALL_DIR="$2"; shift 2 ;;
        *) warn "Unknown option: $1"; shift ;;
    esac
done

[[ -z "$DOMAIN" ]] && error "Missing --domain. Usage: --domain bot.company.vn"
[[ -z "$ADMIN_EMAIL" ]] && ADMIN_EMAIL="admin@${DOMAIN}"
[[ -z "$DB_PASSWORD" ]] && DB_PASSWORD=$(openssl rand -base64 24 | tr -d '=+/' | head -c 20)

echo ""
echo "════════════════════════════════════════════════════"
echo "  ⚡ BizClaw Installer"
echo ""
echo "  Domain:     $DOMAIN"
echo "  Email:      $ADMIN_EMAIL"
echo "  Port:       $BIZCLAW_PORT"
echo "  Install:    $INSTALL_DIR"
echo "════════════════════════════════════════════════════"
echo ""

# ── Check root ───────────────────────────────────────────────

[[ $EUID -ne 0 ]] && error "This script must be run as root (sudo)"

# ── 1. System dependencies ───────────────────────────────────

log "📦 Installing system dependencies..."
apt-get update -qq
apt-get install -y -qq \
    build-essential curl git \
    postgresql postgresql-contrib \
    nginx certbot python3-certbot-nginx \
    > /dev/null 2>&1

# ── 2. PostgreSQL setup ──────────────────────────────────────

log "🐘 Setting up PostgreSQL..."
systemctl enable --now postgresql

sudo -u postgres psql -c "CREATE USER ${BIZCLAW_USER} WITH PASSWORD '${DB_PASSWORD}';" 2>/dev/null || true
sudo -u postgres psql -c "CREATE DATABASE bizclaw OWNER ${BIZCLAW_USER};" 2>/dev/null || true
sudo -u postgres psql -c "GRANT ALL PRIVILEGES ON DATABASE bizclaw TO ${BIZCLAW_USER};" 2>/dev/null || true

log "✅ PostgreSQL ready — user: $BIZCLAW_USER, db: bizclaw"

# ── 3. Create bizclaw user ───────────────────────────────────

if ! id -u "$BIZCLAW_USER" &>/dev/null; then
    useradd -r -m -d "$INSTALL_DIR" -s /bin/bash "$BIZCLAW_USER"
    log "👤 Created system user: $BIZCLAW_USER"
fi

mkdir -p "$INSTALL_DIR"

# ── 4. Download or build BizClaw ─────────────────────────────

log "🔨 Installing BizClaw..."

# Try to download pre-built binary
ARCH=$(uname -m)
BINARY_URL="https://github.com/nguyenduchoai/bizclaw-cloud/releases/latest/download/bizclaw-${ARCH}-linux"
PLATFORM_URL="https://github.com/nguyenduchoai/bizclaw-cloud/releases/latest/download/bizclaw-platform-${ARCH}-linux"

if curl -fsSL -o "$INSTALL_DIR/bizclaw" "$BINARY_URL" 2>/dev/null && \
   curl -fsSL -o "$INSTALL_DIR/bizclaw-platform" "$PLATFORM_URL" 2>/dev/null; then
    chmod +x "$INSTALL_DIR/bizclaw" "$INSTALL_DIR/bizclaw-platform"
    log "✅ Downloaded pre-built binaries"
else
    warn "Pre-built binaries not available. Building from source..."

    # Install Rust if needed
    if ! command -v cargo &>/dev/null; then
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source "$HOME/.cargo/env"
    fi

    cd /tmp
    git clone --depth 1 https://github.com/nguyenduchoai/bizclaw-cloud.git bizclaw-src
    cd bizclaw-src
    cargo build --release
    cp target/release/bizclaw "$INSTALL_DIR/"
    cp target/release/bizclaw-platform "$INSTALL_DIR/"
    cd / && rm -rf /tmp/bizclaw-src

    log "✅ Built from source"
fi

# ── 5. Create config ─────────────────────────────────────────

log "⚙️ Creating config..."
cat > "$INSTALL_DIR/config.toml" << EOF
# BizClaw Configuration — Auto-generated
# Domain: $DOMAIN

[server]
host = "127.0.0.1"
port = $BIZCLAW_PORT

[database]
url = "postgres://${BIZCLAW_USER}:${DB_PASSWORD}@localhost/bizclaw"

[admin]
username = "admin"
# Login at https://$DOMAIN/admin/

[security]
cors_origins = ["https://$DOMAIN"]
EOF

# ── 6. Create systemd service ────────────────────────────────

log "🔧 Creating systemd service..."
cat > /etc/systemd/system/bizclaw.service << EOF
[Unit]
Description=BizClaw AI Agent Platform
After=network.target postgresql.service
Wants=postgresql.service

[Service]
Type=simple
ExecStart=$INSTALL_DIR/bizclaw-platform --config $INSTALL_DIR/config.toml
Restart=always
RestartSec=5
User=$BIZCLAW_USER
WorkingDirectory=$INSTALL_DIR
Environment=DATABASE_URL=postgres://${BIZCLAW_USER}:${DB_PASSWORD}@localhost/bizclaw
LimitNOFILE=65535

# Hardening
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=$INSTALL_DIR
NoNewPrivileges=true
PrivateTmp=true

[Install]
WantedBy=multi-user.target
EOF

# Fix ownership
chown -R "$BIZCLAW_USER:$BIZCLAW_USER" "$INSTALL_DIR"

# Initialize database
sudo -u "$BIZCLAW_USER" "$INSTALL_DIR/bizclaw-platform" init --config "$INSTALL_DIR/config.toml" 2>/dev/null || true

systemctl daemon-reload
systemctl enable --now bizclaw
log "✅ BizClaw service started"

# ── 7. Nginx reverse proxy ───────────────────────────────────

log "🌐 Setting up Nginx..."
cat > /etc/nginx/sites-available/bizclaw << EOF
server {
    listen 80;
    server_name $DOMAIN;

    location / {
        proxy_pass http://127.0.0.1:$BIZCLAW_PORT;
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \$scheme;

        # WebSocket support
        proxy_http_version 1.1;
        proxy_set_header Upgrade \$http_upgrade;
        proxy_set_header Connection "upgrade";

        # Timeouts
        proxy_connect_timeout 60;
        proxy_send_timeout 300;
        proxy_read_timeout 300;
    }

    # Static files cache
    location ~* \.(css|js|png|jpg|jpeg|gif|ico|svg|woff2?)$ {
        proxy_pass http://127.0.0.1:$BIZCLAW_PORT;
        expires 7d;
        add_header Cache-Control "public, immutable";
    }
}
EOF

ln -sf /etc/nginx/sites-available/bizclaw /etc/nginx/sites-enabled/
rm -f /etc/nginx/sites-enabled/default
nginx -t && systemctl reload nginx

# ── 8. SSL Certificate ───────────────────────────────────────

log "🔒 Installing SSL certificate..."
certbot --nginx -d "$DOMAIN" --non-interactive --agree-tos -m "$ADMIN_EMAIL" || \
    warn "SSL certificate failed — you can retry with: certbot --nginx -d $DOMAIN"

# ── 9. Health check ──────────────────────────────────────────

sleep 3
if curl -sf "http://127.0.0.1:$BIZCLAW_PORT/health" > /dev/null 2>&1; then
    log "✅ Health check passed!"
else
    warn "Service may still be starting. Check: systemctl status bizclaw"
fi

# ── Done! ────────────────────────────────────────────────────

echo ""
echo "════════════════════════════════════════════════════════════"
echo "  ✅ BizClaw installed successfully!"
echo ""
echo "  🌐 Dashboard:  https://$DOMAIN"
echo "  🔑 Admin:      https://$DOMAIN/admin/"
echo "  📊 API:        https://$DOMAIN/v1/chat/completions"
echo ""
echo "  📁 Install:    $INSTALL_DIR"
echo "  📋 Config:     $INSTALL_DIR/config.toml"
echo "  🐘 Database:   bizclaw (PostgreSQL)"
echo "  🔐 DB Password: $DB_PASSWORD"
echo ""
echo "  📝 Commands:"
echo "    systemctl status bizclaw     # Check status"
echo "    systemctl restart bizclaw    # Restart"
echo "    journalctl -u bizclaw -f     # View logs"
echo ""
echo "  ⚡ Next steps:"
echo "    1. Login at https://$DOMAIN/admin/"
echo "    2. Create your first tenant"
echo "    3. Configure agents with API keys"
echo "════════════════════════════════════════════════════════════"
