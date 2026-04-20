#!/bin/bash
# BizClaw Quick Install Script
# Supports: macOS, Linux, Windows (WSL)

set -e

echo "🦞 BizClaw Installer v1.1.7"
echo "================================"

# Detect OS
detect_os() {
  if [[ "$OSTYPE" == "darwin"* ]]; then
    echo "macOS detected"
    PKG_MGR="brew"
  elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
    if command -v apt-get &> /dev/null; then
      echo "Linux (apt) detected"
      PKG_MGR="apt"
    elif command -v pacman &> /dev/null; then
      echo "Linux (pacman) detected"
      PKG_MGR="pacman"
    fi
  elif [[ "$OSTYPE" == "msys" ]] || [[ "$OSTYPE" == "cygwin" ]]; then
    echo "Windows detected"
    PKG_MGR="windows"
  fi
}

# Install Rust (if not installed)
install_rust() {
  if ! command -v rustc &> /dev/null; then
    echo "📦 Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
  else
    echo "✅ Rust already installed"
  fi
}

# Clone and build
clone_and_build() {
  echo "📥 Cloning BizClaw..."
  if [ -d "bizclaw" ]; then
    echo "⚠️ Directory bizclaw exists, pulling latest..."
    cd bizclaw && git pull
    cd ..
  else
    git clone https://github.com/nguyenduchoai/bizclaw.git
    cd bizclaw
  fi

  echo "🔨 Building BizClaw (this may take 10-20 minutes)..."
  cargo build --release

  echo "✅ Build complete!"
  echo "   Binary: ./target/release/bizclaw-desktop"
}

# Run
run_bizclaw() {
  echo "🚀 Starting BizClaw..."
  echo "   Web UI: http://localhost:3000"
  echo "   API: http://localhost:8080"
  echo ""
  
  if [ "$PKG_MGR" = "macOS" ] || [ "$PKG_MGR" = "apt" ] || [ "$PKG_MGR" = "pacman" ]; then
    ./target/release/bizclaw-desktop
  else
    ./target/release/bizclaw-desktop.exe
  fi
}

# Main
main() {
  detect_os
  install_rust
  clone_and_build
  run_bizclaw
}

main "$@"
