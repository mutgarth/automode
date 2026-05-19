#!/usr/bin/env bash
# Antigravity PreToolUse hook — forwards tool calls to automode.
# If automode is unreachable or the LLM is slow, exits silently
# so Antigravity falls back to its normal approval flow.

AUTOMODE_URL="http://localhost:7878/decide"

BODY=$(cat)

RESPONSE=$(echo "$BODY" | curl -s \
  --max-time 30 \
  --connect-timeout 2 \
  -X POST \
  -H "Content-Type: application/json" \
  -d @- \
  "$AUTOMODE_URL" 2>/dev/null)

if [ -z "$RESPONSE" ]; then
  exit 0
fi

echo "$RESPONSE"
