#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════
# BizClaw — Auto Version Bump Script
# ═══════════════════════════════════════════════════════════
#
# Usage:
#   ./scripts/bump-version.sh patch   # 1.0.0 → 1.0.1 (default)
#   ./scripts/bump-version.sh minor   # 1.0.0 → 1.1.0
#   ./scripts/bump-version.sh major   # 1.0.0 → 2.0.0
#
# Updates:
#   - Cargo.toml (workspace version)
#   - android/app/build.gradle.kts (versionName + versionCode)
#   - VERSION file (single source of truth)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
CARGO_TOML="$PROJECT_DIR/Cargo.toml"
GRADLE_FILE="$PROJECT_DIR/android/app/build.gradle.kts"
VERSION_FILE="$PROJECT_DIR/VERSION"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log() { echo -e "${BLUE}[VERSION]${NC} $*"; }
ok()  { echo -e "${GREEN}[OK]${NC} $*"; }

# ── Read current version ──────────────────────────────────
if [ -f "$VERSION_FILE" ]; then
    CURRENT=$(cat "$VERSION_FILE" | tr -d '[:space:]')
else
    # Parse from Cargo.toml
    CURRENT=$(grep '^version = ' "$CARGO_TOML" | head -1 | sed 's/version = "\(.*\)"/\1/')
fi

# Parse semver
IFS='.' read -r MAJOR MINOR PATCH <<< "$CURRENT"
BUMP_TYPE="${1:-patch}"

log "Current version: ${YELLOW}$CURRENT${NC}"

# ── Calculate new version ─────────────────────────────────
case "$BUMP_TYPE" in
    major)
        MAJOR=$((MAJOR + 1))
        MINOR=0
        PATCH=0
        ;;
    minor)
        MINOR=$((MINOR + 1))
        PATCH=0
        ;;
    patch|*)
        PATCH=$((PATCH + 1))
        ;;
esac

NEW_VERSION="$MAJOR.$MINOR.$PATCH"
log "New version:     ${GREEN}$NEW_VERSION${NC} ($BUMP_TYPE)"

# ── Update VERSION file ──────────────────────────────────
echo "$NEW_VERSION" > "$VERSION_FILE"
ok "VERSION file updated"

# ── Update Cargo.toml workspace version ──────────────────
if [ -f "$CARGO_TOML" ]; then
    sed -i.bak "s/^version = \"$CURRENT\"/version = \"$NEW_VERSION\"/" "$CARGO_TOML"
    rm -f "$CARGO_TOML.bak"
    ok "Cargo.toml: $CURRENT → $NEW_VERSION"
fi

# ── Update Android build.gradle.kts ─────────────────────
if [ -f "$GRADLE_FILE" ]; then
    # Get current versionCode
    CURRENT_CODE=$(grep 'versionCode = ' "$GRADLE_FILE" | head -1 | sed 's/.*versionCode = \([0-9]*\).*/\1/')
    NEW_CODE=$((CURRENT_CODE + 1))

    # Update versionName
    sed -i.bak "s/versionName = \".*\"/versionName = \"$NEW_VERSION\"/" "$GRADLE_FILE"
    # Update versionCode
    sed -i.bak "s/versionCode = $CURRENT_CODE/versionCode = $NEW_CODE/" "$GRADLE_FILE"
    # Update APP_VERSION buildConfigField
    sed -i.bak "s/APP_VERSION\", \"\\\\\".*\\\\\"\"/APP_VERSION\", \"\\\\\"$NEW_VERSION\\\\\"\"/" "$GRADLE_FILE"
    rm -f "$GRADLE_FILE.bak"
    ok "Android: versionName=$NEW_VERSION, versionCode=$NEW_CODE"
fi

# ── Summary ──────────────────────────────────────────────
echo ""
echo "═══════════════════════════════════════"
echo -e "  📦 Version bumped: ${YELLOW}$CURRENT${NC} → ${GREEN}$NEW_VERSION${NC}"
echo "═══════════════════════════════════════"
echo ""
echo "  Files updated:"
echo "    ✅ VERSION"
echo "    ✅ Cargo.toml (workspace)"
echo "    ✅ android/app/build.gradle.kts"
echo ""
echo "  Next: git add -A && git commit && git push"
echo ""
