# automode

> **Auto-approve Claude Code, Codex, and Antigravity permission prompts using a local LLM — no UI interruptions, ~500 ms decisions, fully private.**

[![Latest release](https://img.shields.io/github/v/release/mutgarth/automode?label=release&color=brightgreen)](https://github.com/mutgarth/automode/releases/latest)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/built%20with-Rust-orange?logo=rust)](https://www.rust-lang.org/)
[![llama.cpp](https://img.shields.io/badge/inference-llama.cpp-green)](https://github.com/ggerganov/llama.cpp)
[![Claude Code](https://img.shields.io/badge/works%20with-Claude%20Code-blueviolet?logo=anthropic)](https://claude.ai/code)
[![Codex](https://img.shields.io/badge/works%20with-Codex-black?logo=openai)](https://github.com/openai/codex)
[![Antigravity](https://img.shields.io/badge/works%20with-Antigravity-4285F4?logo=google)](https://antigravity.google/)

---

Claude Code asks for permission before running shell commands, reading files, calling APIs, etc. With `automode`, those prompts are intercepted by a local LLM that reasons about each tool call and decides in ~500 ms — without any UI prompt, without sending data to the cloud, and without ever making things less safe than vanilla Claude Code.

```
❯ automode start
  ✓ llama-server running  (localhost:8080, Bonsai-8B-Q1_0, Metal)
  ✓ automode daemon running  (localhost:7878, mode: mild)
  Hook registered in ~/.claude/settings.json
```

---

## How it works

```
Claude Code
    │  PreToolUse hook fires
    ▼
~/.automode/hook.sh
    │  POST /decide  (2 s connect timeout, 30 s total — silent fallback on error)
    ▼
automode  (Rust HTTP daemon · localhost:7878)
    │  builds prompt: system = policy, user = tool call JSON
    ▼
llama.cpp server  (subprocess managed by automode · localhost:8080)
    │  returns { "decision": "approve" | "reject", "reason": "..." }
    ▼
automode → hook → Claude Code  (no prompt shown)
```

If the LLM can't decide or the daemon is unreachable, the hook exits with code `0` and Claude Code falls through to its normal permission prompt. **automode never silently breaks anything.**

---

## Modes

| Mode | What the LLM approves |
|---|---|
| `yolo` | Everything except catastrophic ops (`rm -rf ~`, `DROP DATABASE prod`, …) |
| `mild` | Common dev workflow — reads, writes, git, cargo, npm; rejects destructive ops |
| `strict` | Read-only operations only |
| `custom` | Your own `~/.automode/policy.md` injected verbatim as the system prompt |

Every mode still runs the LLM — there is no static bypass list. The model always reasons about the call.

---

## Installation

```sh
curl -fsSL https://raw.githubusercontent.com/mutgarth/automode/main/scripts/install.sh | sh
```

The installer:
1. Downloads the `automode` binary for your platform
2. Downloads `llama-server` from the latest llama.cpp release
3. Downloads `Bonsai-8B-Q1_0.gguf` (~1.16 GB) from Hugging Face
4. Runs `automode setup` — interactive mode selection
5. Patches `~/.claude/settings.json` to register the PreToolUse hook

**After installation, restart any open Claude Code sessions** so they pick up the hook.

### Build from source

```sh
git clone https://github.com/mutgarth/automode
cd automode
cargo build --release
./target/release/automode dev   # downloads llama-server + model, runs setup
```

---

## Commands

```
automode setup              Interactive onboarding — installs hook, picks mode
automode setup --target T   T = claude (default) | codex | antigravity | both | all
automode start              Start the daemon and llama-server in the background
automode stop               Stop everything cleanly
automode status             Show running state, mode, and last decisions
automode mode <name>        Switch to yolo | mild | strict | custom
automode logs               Tail decisions.log
automode dev                Local-build setup (skips GitHub release download)
automode update             Refresh binary/hooks; preserve mode, ports, paths
automode update --target T  Also set integration target: claude | codex | antigravity | both | all
```

After installation, **restart any open Claude Code, Codex, or Antigravity sessions** so they pick up the hook.

---

## Codex and Antigravity

The same daemon can serve Codex (`PermissionRequest` hooks in `~/.codex/hooks.json`) and Antigravity (`PreToolUse` hooks in `~/.gemini/config/hooks.json`) alongside Claude Code. Each integration returns the host's native decision format.

```sh
automode setup --target codex         # Codex only
automode setup --target antigravity   # Antigravity only
automode setup --target both          # Claude Code + Codex
automode setup --target all           # Claude Code + Codex + Antigravity
```

Claude Code remains the default target, so existing installs keep working unchanged.

Run `automode update` after upgrading automode or pulling a newer local build. It refreshes
the installed binary and hook scripts, patches missing host hook files, migrates older
configs to include an integration target, and preserves existing mode, ports, model path,
and log settings. If no target is configured yet, update defaults to `all`.

---

## Custom policies

Switch to `custom` mode and edit `~/.automode/policy.md`. The file is injected verbatim as the LLM system prompt, giving you full control:

```markdown
# My policy

## Always approve
- Anything inside ~/projects/
- Read-only database queries against the staging DB

## Always reject
- Any command that rewrites git history
- Anything touching the production cluster (prod-*)
```

---

## Performance

Tested on Apple Silicon with Metal acceleration:

| Decision type | Latency |
|---|---|
| Simple command (`echo`, `ls`, `cat`) | ~500 ms |
| Complex command (`for` loop, `$(...)`, multi-line) | ~600–700 ms |

The 1-bit Bonsai model is **1.16 GB on disk** and uses **~1 GB RAM** at runtime. It runs entirely on-device — no data leaves your machine.

---

## File layout

```
~/.automode/
  automode            ← the Rust daemon binary
  llama-server        ← llama.cpp server binary
  *.dylib             ← llama.cpp shared libs (macOS)
  hook.sh             ← Claude Code PreToolUse hook
  codex-hook.sh       ← Codex PermissionRequest hook (when --target includes codex)
  antigravity-hook.sh ← Antigravity PreToolUse hook (when --target includes antigravity)
  config.toml         ← port, mode, paths, log level
  policy.md           ← active in custom mode
  models/
    bonsai.gguf       ← ~1.16 GB GGUF model
  logs/
    decisions.log     ← every approve/reject with LLM reasoning
    failures.log      ← cases where the LLM couldn't decide
    llama-server.log  ← llama-server stderr
```

---

## Tech stack

| Layer | Library |
|---|---|
| Language | Rust 2021 |
| HTTP server | axum 0.7 |
| LLM client | reqwest 0.12 |
| Async runtime | tokio 1 |
| CLI | clap 4 |
| LLM inference | llama.cpp |
| Model | Bonsai-8B-Q1\_0.gguf |

---

## License

MIT — see [LICENSE](LICENSE).
