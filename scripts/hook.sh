#!/usr/bin/env bash
# Claude Code PreToolUse hook — forwards tool call to automode daemon.
# If automode is unreachable, exits silently (Claude Code prompts user).

AUTOMODE_URL="http://localhost:7878/decide"
TIMEOUT=2

# Read the tool call JSON from stdin
BODY=$(cat)

# POST to automode with a 2-second timeout
RESPONSE=$(echo "$BODY" | curl -s \
  --max-time "$TIMEOUT" \
  --connect-timeout "$TIMEOUT" \
  -X POST \
  -H "Content-Type: application/json" \
  -d @- \
  "$AUTOMODE_URL" 2>/dev/null)

# If curl failed or response is empty, exit silently (fall through to Claude Code)
if [ -z "$RESPONSE" ]; then
  exit 0
fi

# Output the decision for Claude Code
echo "$RESPONSE"
