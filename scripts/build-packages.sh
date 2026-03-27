#!/bin/bash
# ═══════════════════════════════════════════════════════════════
# BizClaw — Build All Packages (macOS DMG, Linux DEB, Windows MSI)
# Usage: ./scripts/build-packages.sh [--release] [--target <target>]
# ═══════════════════════════════════════════════════════════════
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
VERSION=$(cat "$ROOT_DIR/VERSION" 2>/dev/null || echo "1.0.5")
PROFILE="release"
BUILD_DIR="$ROOT_DIR/target/release"
DIST_DIR="$ROOT_DIR/dist"
APP_NAME="BizClaw"
BIN_NAME="bizclaw-desktop"

echo ""
echo "  🦀 BizClaw Package Builder v${VERSION}"
echo "  ═══════════════════════════════════════"
echo ""

# Parse args
TARGET=""
while [[ $# -gt 0 ]]; do
  case $1 in
    --target) TARGET="$2"; shift 2 ;;
    --release) PROFILE="release"; shift ;;
    *) shift ;;
  esac
done

mkdir -p "$DIST_DIR"

# ── Detect OS ──
OS="$(uname -s)"
ARCH="$(uname -m)"

build_binary() {
  local target="$1"
  echo "  📦 Building $BIN_NAME for $target..."
  if [[ -n "$target" ]]; then
    cargo build --release --bin "$BIN_NAME" --target "$target"
    echo "  ✅ Binary: target/$target/release/$BIN_NAME"
  else
    cargo build --release --bin "$BIN_NAME"
    echo "  ✅ Binary: target/release/$BIN_NAME"
  fi
}

# ═══════════════════════════════════════════════════════════════
# macOS DMG
# ═══════════════════════════════════════════════════════════════
build_dmg() {
  echo ""
  echo "  🍎 Building macOS DMG..."
  echo "  ────────────────────────"
  
  local DMG_DIR="$DIST_DIR/dmg-staging"
  local APP_DIR="$DMG_DIR/${APP_NAME}.app"
  local DMG_NAME="${APP_NAME}-${VERSION}-${ARCH}.dmg"
  
  # Build if not already built
  if [[ ! -f "$BUILD_DIR/$BIN_NAME" ]]; then
    build_binary ""
  fi
  
  # Clean previous
  rm -rf "$DMG_DIR"
  
  # Create .app bundle structure
  mkdir -p "$APP_DIR/Contents/MacOS"
  mkdir -p "$APP_DIR/Contents/Resources"
  
  # Copy binary
  cp "$BUILD_DIR/$BIN_NAME" "$APP_DIR/Contents/MacOS/$APP_NAME"
  chmod +x "$APP_DIR/Contents/MacOS/$APP_NAME"
  
  # Create Info.plist
  cat > "$APP_DIR/Contents/Info.plist" << PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleExecutable</key>
  <string>${APP_NAME}</string>
  <key>CFBundleIdentifier</key>
  <string>vn.bizclaw.desktop</string>
  <key>CFBundleName</key>
  <string>${APP_NAME}</string>
  <key>CFBundleDisplayName</key>
  <string>${APP_NAME} Desktop</string>
  <key>CFBundleVersion</key>
  <string>${VERSION}</string>
  <key>CFBundleShortVersionString</key>
  <string>${VERSION}</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleSignature</key>
  <string>BZCL</string>
  <key>LSMinimumSystemVersion</key>
  <string>11.0</string>
  <key>CFBundleInfoDictionaryVersion</key>
  <string>6.0</string>
  <key>NSHighResolutionCapable</key>
  <true/>
  <key>LSApplicationCategoryType</key>
  <string>public.app-category.developer-tools</string>
  <key>CFBundleIconFile</key>
  <string>AppIcon</string>
</dict>
</plist>
PLIST

  # Create a simple icon (placeholder — generate with iconutil for production)
  if command -v iconutil &>/dev/null && [[ -f "$ROOT_DIR/packaging/macos/AppIcon.iconset/icon_512x512.png" ]]; then
    iconutil -c icns "$ROOT_DIR/packaging/macos/AppIcon.iconset" -o "$APP_DIR/Contents/Resources/AppIcon.icns"
  fi
  
  # Create DMG
  echo "  📀 Creating DMG..."
  if command -v create-dmg &>/dev/null; then
    # Premium DMG with background, icon layout
    create-dmg \
      --volname "${APP_NAME} ${VERSION}" \
      --volicon "$APP_DIR/Contents/Resources/AppIcon.icns" 2>/dev/null \
      --window-pos 200 120 \
      --window-size 600 400 \
      --icon-size 100 \
      --icon "${APP_NAME}.app" 175 190 \
      --hide-extension "${APP_NAME}.app" \
      --app-drop-link 425 190 \
      "$DIST_DIR/$DMG_NAME" \
      "$DMG_DIR" 2>/dev/null || true
  fi
  
  # Fallback: hdiutil
  if [[ ! -f "$DIST_DIR/$DMG_NAME" ]]; then
    hdiutil create -volname "${APP_NAME}" \
      -srcfolder "$DMG_DIR" \
      -ov -format UDZO \
      "$DIST_DIR/$DMG_NAME"
  fi
  
  # Cleanup staging
  rm -rf "$DMG_DIR"
  
  local SIZE=$(du -h "$DIST_DIR/$DMG_NAME" | cut -f1)
  echo "  ✅ DMG: dist/$DMG_NAME ($SIZE)"
}

# ═══════════════════════════════════════════════════════════════
# Linux DEB
# ═══════════════════════════════════════════════════════════════
build_deb() {
  echo ""
  echo "  🐧 Building Linux DEB..."
  echo "  ────────────────────────"
  
  local DEB_DIR="$DIST_DIR/deb-staging"
  local DEB_ARCH="amd64"
  [[ "$ARCH" == "aarch64" || "$ARCH" == "arm64" ]] && DEB_ARCH="arm64"
  local DEB_NAME="bizclaw_${VERSION}_${DEB_ARCH}.deb"
  local LINUX_BIN=""

  # Check if cross-compiled linux binary exists
  if [[ -f "$ROOT_DIR/target/x86_64-unknown-linux-gnu/release/$BIN_NAME" ]]; then
    LINUX_BIN="$ROOT_DIR/target/x86_64-unknown-linux-gnu/release/$BIN_NAME"
  elif [[ -f "$ROOT_DIR/target/aarch64-unknown-linux-gnu/release/$BIN_NAME" ]]; then
    LINUX_BIN="$ROOT_DIR/target/aarch64-unknown-linux-gnu/release/$BIN_NAME"
    DEB_ARCH="arm64"
  elif [[ "$OS" == "Linux" && -f "$BUILD_DIR/$BIN_NAME" ]]; then
    LINUX_BIN="$BUILD_DIR/$BIN_NAME"
  else
    echo "  ⚠️  No Linux binary found. Cross-compile first:"
    echo "     cargo build --release --bin $BIN_NAME --target x86_64-unknown-linux-gnu"
    echo "  ⏭  Skipping DEB build."
    return 0
  fi

  # Clean previous
  rm -rf "$DEB_DIR"

  # Create DEB structure
  mkdir -p "$DEB_DIR/DEBIAN"
  mkdir -p "$DEB_DIR/usr/bin"
  mkdir -p "$DEB_DIR/usr/share/applications"
  mkdir -p "$DEB_DIR/usr/share/icons/hicolor/256x256/apps"
  mkdir -p "$DEB_DIR/usr/share/doc/bizclaw"

  # Copy binary
  cp "$LINUX_BIN" "$DEB_DIR/usr/bin/bizclaw-desktop"
  chmod 755 "$DEB_DIR/usr/bin/bizclaw-desktop"

  # Also create 'bizclaw' symlink for CLI access
  ln -sf bizclaw-desktop "$DEB_DIR/usr/bin/bizclaw"

  # Control file
  cat > "$DEB_DIR/DEBIAN/control" << CTRL
Package: bizclaw
Version: ${VERSION}
Section: utils
Priority: optional
Architecture: ${DEB_ARCH}
Maintainer: BizClaw Team <hello@bizclaw.vn>
Description: BizClaw Desktop — AI Agent Platform
 Full-featured AI assistant with built-in dashboard,
 multi-agent orchestration, and 80+ API endpoints.
 Runs standalone with embedded web UI.
Homepage: https://bizclaw.vn
Depends: libssl3 | libssl1.1, ca-certificates
Installed-Size: $(du -sk "$LINUX_BIN" | cut -f1)
CTRL

  # Post-install script
  cat > "$DEB_DIR/DEBIAN/postinst" << 'POSTINST'
#!/bin/sh
set -e
echo ""
echo "  🦀 BizClaw Desktop installed!"
echo "  Run: bizclaw-desktop"
echo "  Or:  bizclaw serve"
echo ""
POSTINST
  chmod 755 "$DEB_DIR/DEBIAN/postinst"

  # Desktop entry
  cat > "$DEB_DIR/usr/share/applications/bizclaw.desktop" << DESKTOP
[Desktop Entry]
Name=BizClaw Desktop
Comment=AI Agent Platform
Exec=bizclaw-desktop
Terminal=false
Type=Application
Categories=Development;Utility;
Icon=bizclaw
StartupWMClass=bizclaw
DESKTOP

  # Build DEB
  if command -v dpkg-deb &>/dev/null; then
    dpkg-deb --build --root-owner-group "$DEB_DIR" "$DIST_DIR/$DEB_NAME"
    echo "  ✅ DEB: dist/$DEB_NAME"
  elif command -v fakeroot &>/dev/null; then
    fakeroot dpkg-deb --build "$DEB_DIR" "$DIST_DIR/$DEB_NAME"
    echo "  ✅ DEB: dist/$DEB_NAME"
  else
    echo "  ⚠️  dpkg-deb not found. Creating tar.gz instead."
    tar -czf "$DIST_DIR/bizclaw-${VERSION}-linux-${DEB_ARCH}.tar.gz" -C "$DEB_DIR/usr/bin" bizclaw-desktop
    echo "  ✅ TAR: dist/bizclaw-${VERSION}-linux-${DEB_ARCH}.tar.gz"
  fi

  rm -rf "$DEB_DIR"
}

# ═══════════════════════════════════════════════════════════════
# Windows MSI (via WiX or cargo-wix)
# ═══════════════════════════════════════════════════════════════
build_msi() {
  echo ""
  echo "  🪟 Building Windows MSI..."
  echo "  ────────────────────────"
  
  local WIN_BIN=""
  local MSI_NAME="${APP_NAME}-${VERSION}-x64.msi"

  # Check for cross-compiled Windows binary
  if [[ -f "$ROOT_DIR/target/x86_64-pc-windows-gnu/release/${BIN_NAME}.exe" ]]; then
    WIN_BIN="$ROOT_DIR/target/x86_64-pc-windows-gnu/release/${BIN_NAME}.exe"
  elif [[ -f "$ROOT_DIR/target/x86_64-pc-windows-msvc/release/${BIN_NAME}.exe" ]]; then
    WIN_BIN="$ROOT_DIR/target/x86_64-pc-windows-msvc/release/${BIN_NAME}.exe"
  elif [[ "$OS" == *"MINGW"* || "$OS" == *"MSYS"* ]] && [[ -f "$BUILD_DIR/${BIN_NAME}.exe" ]]; then
    WIN_BIN="$BUILD_DIR/${BIN_NAME}.exe"
  else
    echo "  ⚠️  No Windows binary found. Cross-compile first:"
    echo "     cargo build --release --bin $BIN_NAME --target x86_64-pc-windows-gnu"
    echo "  ⏭  Skipping MSI build."
    
    # Create WiX template for later use
    cat > "$ROOT_DIR/packaging/windows/bizclaw.wxs" << 'WXS'
<?xml version="1.0" encoding="UTF-8"?>
<Wix xmlns="http://wixtoolset.org/schemas/v4/wxs">
  <Package Name="BizClaw Desktop"
           Manufacturer="BizClaw Team"
           Version="$(var.Version)"
           UpgradeCode="8E7A5D3C-4F2B-1A9E-0B6D-7C8F3E2D1A0B">
    <MajorUpgrade DowngradeErrorMessage="Phiên bản mới hơn đã được cài đặt." />
    <Feature Id="Main">
      <ComponentGroupRef Id="BinComponents" />
    </Feature>
  </Package>

  <Fragment>
    <StandardDirectory Id="ProgramFiles6432Folder">
      <Directory Id="INSTALLFOLDER" Name="BizClaw">
        <Component Id="MainExe" Guid="*">
          <File Source="$(var.BinDir)\bizclaw-desktop.exe" KeyPath="yes" />
        </Component>
        <Component Id="PathEntry" Guid="*">
          <Environment Id="PATH" Name="PATH" Value="[INSTALLFOLDER]" Permanent="no" Part="last" Action="set" System="yes" />
        </Component>
      </Directory>
    </StandardDirectory>
    <ComponentGroup Id="BinComponents">
      <ComponentRef Id="MainExe" />
      <ComponentRef Id="PathEntry" />
    </ComponentGroup>
  </Fragment>
</Wix>
WXS
    echo "  📝 Created WiX template: packaging/windows/bizclaw.wxs"
    return 0
  fi

  # If we have the binary and WiX tools
  if command -v wix &>/dev/null; then
    wix build "$ROOT_DIR/packaging/windows/bizclaw.wxs" \
      -d "Version=$VERSION" \
      -d "BinDir=$(dirname "$WIN_BIN")" \
      -o "$DIST_DIR/$MSI_NAME"
    echo "  ✅ MSI: dist/$MSI_NAME"
  else
    # Just copy the exe with a zip
    local ZIP_NAME="BizClaw-${VERSION}-windows-x64.zip"
    mkdir -p "$DIST_DIR/win-staging"
    cp "$WIN_BIN" "$DIST_DIR/win-staging/bizclaw-desktop.exe"
    cd "$DIST_DIR/win-staging" && zip -9 "../$ZIP_NAME" bizclaw-desktop.exe && cd "$ROOT_DIR"
    rm -rf "$DIST_DIR/win-staging"
    echo "  ✅ ZIP: dist/$ZIP_NAME"
  fi
}

# ═══════════════════════════════════════════════════════════════
# Main
# ═══════════════════════════════════════════════════════════════
cd "$ROOT_DIR"

echo "  🏗️ Version: $VERSION"
echo "  🏗️ OS: $OS ($ARCH)"
echo "  🏗️ Profile: $PROFILE"
echo ""

# Build native binary first
build_binary "${TARGET}"

# Build packages based on OS
case "$OS" in
  Darwin)
    build_dmg
    build_deb   # Attempt cross-platform
    build_msi   # Attempt cross-platform
    ;;
  Linux)
    build_deb
    build_msi   # Attempt cross-platform
    ;;
  MINGW*|MSYS*|CYGWIN*)
    build_msi
    ;;
  *)
    echo "  ⚠️  Unknown OS: $OS"
    ;;
esac

echo ""
echo "  ═══════════════════════════════════════"
echo "  📦 All packages in: dist/"
ls -lh "$DIST_DIR/" 2>/dev/null | grep -v "^total\|staging" | awk '{print "     " $NF " (" $5 ")"}'
echo "  ═══════════════════════════════════════"
echo ""
