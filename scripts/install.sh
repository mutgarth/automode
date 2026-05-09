#!/usr/bin/env bash
set -euo pipefail

REPO="https://github.com/YOUR_USERNAME/automode"
AUTOMODE_DIR="$HOME/.automode"
MODELS_DIR="$AUTOMODE_DIR/models"
LOGS_DIR="$AUTOMODE_DIR/logs"

# Detect platform
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS-$ARCH" in
  Darwin-arm64)   PLATFORM="macos-arm64" ;;
  Darwin-x86_64)  PLATFORM="macos-x86_64" ;;
  Linux-x86_64)   PLATFORM="linux-x86_64" ;;
  *)
    echo "Unsupported platform: $OS-$ARCH"
    exit 1
    ;;
esac

echo ""
echo "Installing automode for $PLATFORM"
echo "──────────────────────────────────────"

# Create directories
mkdir -p "$AUTOMODE_DIR" "$MODELS_DIR" "$LOGS_DIR"

# Download automode binary
echo "→ Downloading automode binary..."
curl -fsSL --progress-bar \
  "$REPO/releases/latest/download/automode-$PLATFORM" \
  -o "$AUTOMODE_DIR/automode"
chmod +x "$AUTOMODE_DIR/automode"

# Download llama-server binary
echo "→ Downloading llama-server binary..."
curl -fsSL --progress-bar \
  "$REPO/releases/latest/download/llama-server-$PLATFORM" \
  -o "$AUTOMODE_DIR/llama-server"
chmod +x "$AUTOMODE_DIR/llama-server"

# Download model (bonsai 1-bit GGUF, ~1GB)
MODEL_FILE="$MODELS_DIR/bonsai.gguf"
if [ -f "$MODEL_FILE" ]; then
  echo "→ Model already present, skipping download."
else
  echo "→ Downloading model (~1GB, this may take a minute)..."
  curl -fsSL --progress-bar \
    "$REPO/releases/latest/download/bonsai.gguf" \
    -o "$MODEL_FILE"
fi

# Add ~/.automode to PATH in shell profile if needed
PROFILE=""
if [ -f "$HOME/.zshrc" ]; then PROFILE="$HOME/.zshrc"
elif [ -f "$HOME/.bashrc" ]; then PROFILE="$HOME/.bashrc"
elif [ -f "$HOME/.bash_profile" ]; then PROFILE="$HOME/.bash_profile"
fi

if [ -n "$PROFILE" ] && ! grep -q 'automode' "$PROFILE"; then
  echo "" >> "$PROFILE"
  echo 'export PATH="$HOME/.automode:$PATH"' >> "$PROFILE"
  echo "→ Added ~/.automode to PATH in $PROFILE"
fi

export PATH="$HOME/.automode:$PATH"

echo ""
echo "──────────────────────────────────────"
echo "Running setup..."
echo ""
automode setup
