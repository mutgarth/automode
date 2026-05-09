# Automode — Design Spec
**Date:** 2026-05-09  
**Status:** Approved

## Overview

A Rust-based local service that automatically answers Claude Code permission prompts using a local LLM. Integrates via the official Claude Code `PreToolUse` hooks system — no PTY interception, no fragile terminal tricks. When the service is unreachable, Claude Code falls through to its normal interactive prompt.

---

## Architecture

Three components:

```
Claude Code
    │  PreToolUse hook fires
    ▼
~/.automode/hook.sh
    │  POST /decide (2s timeout — silent exit if unreachable)
    ▼
automode (Rust HTTP daemon, localhost:7878)
    │  builds prompt: system=policy, user=tool call JSON
    ▼
llama.cpp server (subprocess managed by automode)
    │  returns {"decision": "approve"|"reject", "reason": "..."}
    ▼
automode → hook → Claude Code
```

### Components

**automode** — Rust binary. Responsibilities:
- HTTP server on `localhost:7878`
- Manages llama.cpp server as a child process (starts on `automode start`, stops on `automode stop`)
- Accepts `POST /decide` with Claude Code's tool call JSON
- Builds LLM prompt from active policy + tool call details
- Returns structured decision to the hook
- Writes every decision to `~/.automode/logs/decisions.log`

**hook.sh** — Shell script registered as Claude Code `PreToolUse` hook. Responsibilities:
- Reads tool call JSON from stdin
- POSTs to `localhost:7878/decide` with a 2-second timeout
- On success: outputs `{"decision": "approve"}` or `{"decision": "reject", "reason": "..."}` to stdout
- On timeout/error: exits silently (Claude Code handles it normally)

**llama.cpp server** — Subprocess. Runs the GGUF model (bonsai, ~1GB). Communicates via its OpenAI-compatible HTTP API on `localhost:8080` (internal, not user-facing).

---

## Modes

Four operating modes, selectable at any time with `automode mode <name>`:

| Mode | Behavior |
|------|----------|
| `yolo` | Approve everything. LLM not invoked. Instant. |
| `mild` | Approve common safe ops, reject destructive ones. Uses built-in policy prompt. |
| `strict` | Approve read-only operations only. Uses built-in policy prompt. |
| `custom` | Uses `~/.automode/policy.md` verbatim as the LLM system prompt. |

Preset modes (`yolo`, `mild`, `strict`) have policy prompts baked into the binary — they cannot be accidentally modified.

In `yolo` mode the LLM is not consulted — the service approves immediately, keeping llama.cpp idle.

---

## Policy

### Preset policy (used by `mild`, `strict`, and as starter for `custom`)

```markdown
# Automode Policy

## Always approve
- File reads: ls, cat, find, grep, head, tail
- Git reads: git status, git log, git diff, git branch
- Build/check: cargo check, cargo build, npm install
- Database reads: SELECT, EXPLAIN queries

## Always reject
- Schema changes: DROP, ALTER, TRUNCATE
- Force operations: git push --force, git reset --hard
- Recursive deletes: rm -rf

## Use judgment for everything else
Consider context: is this a dev environment? Is the path sensitive?
When uncertain, reject and explain why.
```

### Custom mode

`automode mode custom` creates `~/.automode/policy.md` from the starter template above if it doesn't exist, then opens it in `$EDITOR`. The file is injected verbatim as the LLM system prompt on every request.

---

## Data Flow (single decision)

1. Claude Code fires `PreToolUse` hook, writes to hook's stdin:
   ```json
   {"tool": "Bash", "input": {"command": "PGPASSWORD=... psql -c \"SELECT COUNT(*)...\""}}
   ```
2. `hook.sh` POSTs JSON body to `localhost:7878/decide` (2s timeout)
3. `automode` selects active policy, builds prompt:
   - **system**: policy text (preset or `policy.md`)
   - **user**: raw tool call JSON
4. Calls llama.cpp via OpenAI-compatible API, expects:
   ```json
   {"decision": "approve", "reason": "Read-only SELECT query"}
   ```
5. Appends to `decisions.log`: timestamp, tool, command, decision, reason
6. Returns decision to hook
7. Hook writes `{"decision": "approve"}` (or reject) to stdout → Claude Code proceeds or blocks

---

## CLI

```
automode setup          # interactive onboarding (installs hook, selects mode)
automode start          # start daemon + llama.cpp
automode stop           # stop daemon + llama.cpp
automode status         # mode, uptime, total decisions, last 5 entries
automode mode <name>    # switch mode (yolo|mild|strict|custom)
automode logs           # tail decisions.log
```

### Onboarding (`automode setup`)

```
Welcome to automode
───────────────────────────────────────
? Select a mode:
  ❯ yolo   — approve everything, no questions asked
    mild   — approves reads/queries, blocks destructive ops
    strict — approves only read-only operations
    custom — write your own policy in policy.md
───────────────────────────────────────
✓ Hook installed in ~/.claude/settings.json
✓ Service configured. Run `automode start` to begin.
```

---

## File Layout

```
~/.automode/
  automode          ← Rust daemon binary
  llama-server      ← llama.cpp server binary
  hook.sh           ← registered in Claude Code settings
  config.toml       ← port, mode, model path, log level
  policy.md         ← active only in custom mode
  models/
    bonsai.gguf     ← ~1GB GGUF model
  logs/
    decisions.log
```

### config.toml
```toml
port = 7878
mode = "mild"
model_path = "~/.automode/models/bonsai.gguf"
llama_server_bin = "~/.automode/llama-server"
llama_server_port = 8080
log_level = "info"
```

### Claude Code hook registration (`~/.claude/settings.json`)
```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": ".*",
        "hooks": [{"type": "command", "command": "~/.automode/hook.sh"}]
      }
    ]
  }
}
```

---

## Installation

One-liner:
```sh
curl -fsSL https://raw.githubusercontent.com/<user>/automode/main/install.sh | sh
```

The install script:
1. Detects platform (macOS arm64/x86_64, Linux x86_64)
2. Downloads `automode` binary from GitHub Releases
3. Downloads `llama-server` binary (pre-built for platform)
4. Downloads `bonsai.gguf` model with progress bar
5. Places all files in `~/.automode/`
6. Runs `automode setup` (interactive onboarding)

---

## Error Handling

| Scenario | Behavior |
|----------|----------|
| Service not running | Hook exits silently → Claude Code prompts user |
| 2s timeout | Hook exits silently → Claude Code prompts user |
| llama.cpp crash | automode restarts it (3 attempts), then returns 503 → hook exits silently |
| LLM returns malformed JSON | Log warning, default to reject |
| LLM returns unknown decision value | Log warning, default to reject |

Fail-safe default: **when in doubt, reject** (except unreachable service → fall through).

---

## Out of Scope

- Remote/cloud LLM support
- Multi-user or networked deployment
- Web UI for logs or policy editing
- Windows support (initial release)
