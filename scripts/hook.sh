#!/usr/bin/env bash
# Claude Code PreToolUse hook — forwards tool call to automode daemon.
# If automode is unreachable or the LLM is slow, exits silently
# (Claude Code prompts user as if no hook were installed).

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
