# automode

A local Rust daemon that auto-approves Claude Code, Codex, and Antigravity permission prompts using a local LLM.

Claude Code asks for permission before running shell commands, editing files, etc. `automode` intercepts these prompts via the official `PreToolUse` hooks system, sends the tool call to a local LLM (Bonsai-8B running on llama.cpp), and lets the LLM decide whether to approve, reject, or fall through to the user.

Codex support uses Codex's `PermissionRequest` hooks in `~/.codex/hooks.json` and returns Codex's hook-specific `allow`/`deny` decision format.

Antigravity support uses Antigravity's `PreToolUse` hooks in `~/.gemini/config/hooks.json` and returns Antigravity's `allow`/`deny` decision format.

The result: most decisions happen in ~500ms with no UI prompt. Catastrophic commands (e.g. `rm -rf ~`, `DROP DATABASE prod`) still get blocked. If the LLM can't decide, the prompt falls through normally — automode never makes things less safe than vanilla Claude Code.

## Architecture

```
Claude Code
    │  PreToolUse hook fires
    ▼
~/.automode/hook.sh
    │  POST /decide  (2s connect, 30s total — silent fallback)
    ▼
automode (Rust HTTP daemon, localhost:7878)
    │  builds prompt: system=policy, user=tool call
    ▼
llama.cpp server (subprocess managed by automode, localhost:8080)
    │  returns JSON {"decision":"approve|reject","reason":"..."}
    ▼
automode → hook → Claude Code (no prompt shown)
```

## Modes

| Mode | Policy |
|------|--------|
| `yolo` | Approve everything except catastrophic ops (LLM is the safety guard) |
| `mild` | Approve common dev workflow, reject destructive ops |
| `strict` | Approve only read-only operations |
| `custom` | Use your own `~/.automode/policy.md` as the LLM system prompt |

Every mode runs the LLM. There is no bypass — the local LLM always reasons about the call.

## Installation

```sh
curl -fsSL https://raw.githubusercontent.com/mutgarth/automode/main/scripts/install.sh | sh
```

The installer:
1. Downloads the `automode` binary for your platform
2. Downloads `llama-server` from the latest llama.cpp release
3. Downloads `Bonsai-8B-Q1_0.gguf` (~1.16 GB) from Hugging Face
4. Runs `automode setup` for interactive mode selection
5. Patches `~/.claude/settings.json` to register the hook

For local development without a GitHub release:

```sh
git clone https://github.com/mutgarth/automode
cd automode
cargo build --release
./target/release/automode dev   # downloads llama-server + model, runs setup
```

## Commands

```
automode setup       # Interactive onboarding — installs hook, picks mode
automode setup --target codex
automode setup --target both
automode setup --target antigravity
automode setup --target all
automode start       # Start the daemon and llama-server in the background
automode stop        # Stop everything cleanly
automode status      # Show running state, mode, last decisions
automode mode <name> # Switch to yolo | mild | strict | custom
automode logs        # Tail decisions.log
automode dev         # Local-build setup (alternative to install.sh)
```

After installation, **restart any open Claude Code, Codex, or Antigravity sessions** so they pick up the hook.

## Codex setup

For Codex, run:

```sh
automode setup --target codex
automode start
```

This installs `~/.automode/codex-hook.sh` and registers it in `~/.codex/hooks.json` under the `PermissionRequest` event. Claude Code remains the default target, so existing installs can continue using:

```sh
automode setup --target claude
```

To install both integrations against the same daemon:

```sh
automode setup --target both
```

## Antigravity setup

For Antigravity, run:

```sh
automode setup --target antigravity
automode start
```

This installs `~/.automode/antigravity-hook.sh` and registers it in `~/.gemini/config/hooks.json` under the `PreToolUse` event.

To install Claude Code, Codex, and Antigravity against the same daemon:

```sh
automode setup --target all
```

## File layout

```
~/.automode/
  automode               ← this Rust binary
  llama-server           ← llama.cpp server binary
  *.dylib                ← llama.cpp shared libraries (macOS)
  hook.sh                ← Claude Code PreToolUse hook
  codex-hook.sh          ← Codex PermissionRequest hook
  antigravity-hook.sh    ← Antigravity PreToolUse hook
  config.toml            ← port, mode, paths, log level
  policy.md              ← active in custom mode
  models/
    bonsai.gguf          ← ~1.16 GB GGUF model
  logs/
    decisions.log        ← every approve/reject with LLM reasoning
    failures.log         ← cases where the LLM couldn't decide
    llama-server.log     ← llama-server stderr
```

## Custom policies

Switch to `custom` mode and edit `~/.automode/policy.md` — the file is injected verbatim as the LLM system prompt. Example:

```markdown
# My policy

## Always approve
- Anything in ~/projects/
- Read-only database queries

## Reject
- Anything that deletes git history
- Anything touching the production cluster
```

## Performance

On Apple Silicon with Metal:

| Decision type | Latency |
|---|---|
| Simple command (`echo`, `ls`) | ~500 ms |
| Complex command (`for` loop, `$(...)`, multi-line) | ~600-700 ms |

The 1-bit Bonsai model is 1.16 GB on disk and uses ~1 GB of RAM at runtime.

## Tech stack

- Rust 2021 edition
- axum 0.7 (HTTP server)
- reqwest 0.12 (LLM client)
- tokio 1 (async runtime)
- clap 4 (CLI)
- llama.cpp (LLM inference)
- Bonsai-8B-Q1_0.gguf (quantized model)

## License

MIT
