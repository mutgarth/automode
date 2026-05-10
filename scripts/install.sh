#!/usr/bin/env bash
set -euo pipefail

AUTOMODE_REPO="https://github.com/mutgarth/automode"
LLAMA_REPO="https://github.com/ggml-org/llama.cpp"
MODEL_URL="https://huggingface.co/prism-ml/Bonsai-8B-gguf/resolve/main/Bonsai-8B-Q1_0.gguf"

AUTOMODE_DIR="$HOME/.automode"
MODELS_DIR="$AUTOMODE_DIR/models"
LOGS_DIR="$AUTOMODE_DIR/logs"

# Detect platform
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS-$ARCH" in
  Darwin-arm64)   PLATFORM="macos-arm64";  LLAMA_PLATFORM="macos-arm64" ;;
  Darwin-x86_64)  PLATFORM="macos-x86_64"; LLAMA_PLATFORM="macos-x86_64" ;;
  Linux-x86_64)   PLATFORM="linux-x86_64"; LLAMA_PLATFORM="ubuntu-x64" ;;
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

# ── automode binary ───────────────────────────────────────────────────────────
if [ -f "$AUTOMODE_DIR/automode" ]; then
  echo "→ automode binary already present, skipping."
else
  echo "→ Downloading automode binary..."
  curl -fsSL --progress-bar \
    "$AUTOMODE_REPO/releases/latest/download/automode-$PLATFORM" \
    -o "$AUTOMODE_DIR/automode"
  chmod +x "$AUTOMODE_DIR/automode"
fi

# ── llama-server binary (from official llama.cpp releases) ───────────────────
if [ -f "$AUTOMODE_DIR/llama-server" ]; then
  echo "→ llama-server already present, skipping."
else
  echo "→ Fetching latest llama.cpp release tag..."
  LLAMA_TAG=$(curl -sL https://api.github.com/repos/ggml-org/llama.cpp/releases/latest \
    | grep '"tag_name"' | head -1 | cut -d'"' -f4)

  if [ -z "$LLAMA_TAG" ]; then
    echo "Error: could not fetch llama.cpp release tag (GitHub API rate limit?)"
    exit 1
  fi

  echo "→ Downloading llama-server $LLAMA_TAG for $PLATFORM..."
  TMP_ZIP=$(mktemp /tmp/llama-XXXXXX.zip)
  curl -fsSL --progress-bar \
    "$LLAMA_REPO/releases/download/$LLAMA_TAG/llama-$LLAMA_TAG-bin-$LLAMA_PLATFORM.zip" \
    -o "$TMP_ZIP"

  # Extract just the llama-server binary
  unzip -j "$TMP_ZIP" "*/llama-server" -d "$AUTOMODE_DIR" 2>/dev/null \
    || unzip -j "$TMP_ZIP" "llama-server" -d "$AUTOMODE_DIR"
  chmod +x "$AUTOMODE_DIR/llama-server"
  rm -f "$TMP_ZIP"
fi

# ── Bonsai-8B GGUF model (1.16 GB) ───────────────────────────────────────────
MODEL_FILE="$MODELS_DIR/bonsai.gguf"
if [ -f "$MODEL_FILE" ]; then
  echo "→ Model already present, skipping download."
else
  echo "→ Downloading Bonsai-8B-Q1_0.gguf (~1.16 GB)..."
  curl -fsSL --progress-bar \
    -H "User-Agent: Mozilla/5.0" \
    "$MODEL_URL" \
    -o "$MODEL_FILE"
fi

# ── PATH ──────────────────────────────────────────────────────────────────────
PROFILE=""
if [ -f "$HOME/.zshrc" ]; then PROFILE="$HOME/.zshrc"
elif [ -f "$HOME/.bashrc" ]; then PROFILE="$HOME/.bashrc"
elif [ -f "$HOME/.bash_profile" ]; then PROFILE="$HOME/.bash_profile"
fi

if [ -n "$PROFILE" ] && ! grep -q '.automode' "$PROFILE"; then
  printf '\nexport PATH="$HOME/.automode:$PATH"\n' >> "$PROFILE"
  echo "→ Added ~/.automode to PATH in $PROFILE"
fi

export PATH="$HOME/.automode:$PATH"

echo ""
echo "──────────────────────────────────────"
echo "Running setup..."
echo ""
automode setup
