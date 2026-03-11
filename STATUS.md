# RockBot Status

**Last Updated:** 2026-03-10

## Build Status

✅ `cargo check` — Compiles with all feature combinations:
  - Default features (`bedrock`, `discord`, `telegram`, `signal`, `tools-*`)
  - `--no-default-features`
  - `--features all-providers,all-channels,all-tools`

✅ `cargo test` — 107 tests total:
  - 71 pass across all crates (excluding rockbot-credentials)
  - 36 pass in rockbot-credentials (17 pre-existing failures in vault storage/manager tests)

## Codebase Overview

- **17 crates** in workspace
- **67 Rust source files**, ~31,200 LOC
- **18 TODOs** remaining in source

| Crate | LOC | Purpose |
|-------|-----|---------|
| `rockbot-cli` | 10,777 | TUI, CLI commands, gateway startup |
| `rockbot-core` | 9,292 | Gateway server, agent engine, sessions, config |
| `rockbot-credentials` | 4,446 | Encrypted credential vault, permissions, audit |
| `rockbot-llm` | 2,399 | LLM provider trait, Anthropic/OpenAI/Bedrock |
| `rockbot-tools` | 1,385 | Tool trait, registry, built-in tools |
| `rockbot-channels-discord` | 596 | Discord channel (Serenity) |
| `rockbot-channels` | 457 | Channel trait, registry, manager |
| `rockbot-channels-telegram` | 442 | Telegram channel (Teloxide) |
| `rockbot-memory` | 390 | Memory manager, keyword search |
| `rockbot-security` | 330 | Capability system, security contexts |
| `rockbot-plugins` | 184 | Plugin manager (scaffold) |
| `rockbot-channels-signal` | 147 | Signal channel (placeholder) |
| `rockbot-tools-mcp` | 114 | MCP server connection tool |
| `rockbot-tools-markdown` | 90 | Markdown processing tool |
| `rockbot-tools-credentials` | 83 | Credential vault access tool |
| `rockbot-credentials-schema` | 64 | Shared credential schema types |
| `rockbot` | 10 | Binary entry point |

---

## Architecture

### Plugin System

All providers (LLM, Channel, Tool) are **self-contained plugins** that register their own credential schemas. The gateway dynamically collects schemas from three registries at startup:

```
rockbot-credentials-schema (leaf crate, only serde)
    ↑                ↑               ↑
rockbot-llm    rockbot-channels   rockbot-tools     ← trait + registry crates
    ↑                ↑               ↑
  bedrock    channels-discord   tools-mcp           ← per-provider crates (optional deps)
  anthropic  channels-telegram  tools-credentials
  openai     channels-signal    tools-markdown
```

**No cyclic dependencies.** Trait crates define interfaces. Per-provider crates implement them. Registration happens in `Gateway::new()` via `#[cfg(feature = "...")]` guards.

### Feature Flags

| Flag | Default | Crate |
|------|---------|-------|
| `bedrock` | ✅ | AWS Bedrock via Converse API |
| `anthropic` | ❌ | Claude via Claude Code SDK (OAuth) |
| `openai` | ❌ | OpenAI models |
| `discord` | ✅ | Discord channel |
| `telegram` | ✅ | Telegram channel |
| `signal` | ✅ | Signal channel (placeholder) |
| `tools-credentials` | ✅ | Credential vault tool |
| `tools-mcp` | ✅ | MCP server tool |
| `tools-markdown` | ✅ | Markdown processing tool |

Feature passthrough: `rockbot` → `rockbot-cli`/`rockbot-core` → per-crate.

### Gateway-Centric Design

The gateway is the **single source of truth** for all runtime state. TUI, WebUI, and CLI are presentation layers that query the gateway API.

- Providers: loaded at startup, queryable via `/api/providers`
- Agents: owned by gateway, persisted to TOML config + per-agent directories
- Credentials: managed via vault, exposed through `/api/credentials/*`
- Schemas: dynamically collected from all registered plugins

---

## What's Working

### Gateway Server (`rockbot-core`)

- **HTTP API** (hyper-based) with 30+ endpoints
- **WebSocket** upgrade placeholder
- **Agent CRUD** — create, update, delete, list with full config persistence
- **Agent execution** — message processing, multi-tool calling, retry with exponential backoff
- **Session management** — SQLite-backed, CRUD, message history, token tracking
- **Config system** — TOML parsing, env var expansion, hot-reload watcher
- **Credential management** — vault integration, auto-unlock, full CRUD API
- **Dynamic provider registration** — LLM, Channel, and Tool schemas collected at startup
- **Agent directory management** — per-agent `SOUL.md` and `SYSTEM-PROMPT.md` files
- **Web UI** — embedded HTML with cyberpunk theme, 6 navigation sections

### Agent Engine (`rockbot-core/agent.rs`)

- System prompt assembly from `SOUL.md`, `AGENTS.md`, skills section
- Multi-turn tool execution loop with configurable iteration limits
- LLM retry with exponential backoff, jitter, and error classification
- Context compaction for long conversations
- Token usage tracking and statistics

### LLM Providers (`rockbot-llm`)

| Provider | Status | Auth |
|----------|--------|------|
| AWS Bedrock | ✅ Working | AWS credentials (env/profile) |
| Anthropic | ✅ Working | Claude Code OAuth or API key |
| OpenAI | ✅ Working | API key |
| Mock | ✅ Testing | None |

- Provider registry with model routing (`get_provider_for_model`)
- Credential schemas self-registered per provider
- Chat completion request/response types with tool calling support

### Channels

| Channel | Status | Notes |
|---------|--------|-------|
| Discord | ✅ Implemented | Serenity-based, embeds, events, self-registering schema |
| Telegram | ✅ Implemented | Teloxide-based, self-registering schema |
| Signal | 🟡 Placeholder | Schema registered, `connect()` returns not-yet-implemented |

- `Channel` trait with `credential_schema()` for self-registration
- `ChannelRegistry` collects schemas from all registered channels
- `ChannelManager` for multi-channel coordination

### Tools

| Tool | Status | Notes |
|------|--------|-------|
| `read` | ✅ Complete | File reading with offset/limit |
| `write` | ✅ Complete | File writing |
| `edit` | ✅ Complete | Text editing |
| `exec` | ✅ Complete | Shell execution |
| `glob` | ✅ Complete | File pattern matching |
| `grep` | ✅ Complete | Content searching |
| `patch` | ✅ Complete | Diff application |
| `memory_get` | ✅ Complete | Memory retrieval (full profile) |
| `memory_search` | ✅ Complete | Memory search (full profile) |

Tool provider crates with self-registering credential schemas:
- `rockbot-tools-credentials` — vault access tool
- `rockbot-tools-mcp` — MCP server connection tool
- `rockbot-tools-markdown` — markdown processing tool

### Credentials (`rockbot-credentials`)

- AES-256-GCM encryption at rest
- Master key derivation via Argon2id
- Multiple unlock methods: password, keyfile, Age, SSH key
- 4-tier permission levels: Allow, AllowHIL, AllowHIL2FA, Deny
- Glob pattern matching for path-based permissions
- HIL (Human-in-the-Loop) approval queue
- Hash-chained tamper-evident audit log
- Full HTTP API (15 endpoints)
- CLI commands: add, list, remove, permissions, audit, status, unlock, lock

### TUI (`rockbot-cli`)

| Section | Status | Notes |
|---------|--------|-------|
| Dashboard | ✅ | Gateway status, agent list, vault status |
| Credentials | ✅ | 4 sub-tabs, full CRUD, unlock flows, dynamic provider schemas |
| Agents | ✅ | List, create, edit — loads from gateway API with config fallback |
| Sessions | 🟡 ~60% | List works, chat partial |
| Models | ✅ | Dynamic provider list from gateway |
| Settings | 🟡 ~40% | Gateway control (start/stop/restart), basic display |

- Elm-like architecture: State → Message → Update → View
- Async data loading via `tokio::spawn` + `mpsc`
- Gateway liveness check with `/api/status`
- Agent save goes through gateway API, falls back to direct config edit offline

### Web UI (`rockbot-core/web_ui.rs`)

- Embedded HTML served from gateway
- 6 navigation sections matching TUI
- Cyberpunk dark theme
- Credential management (init, unlock, endpoints CRUD)
- Dynamic provider cards
- Chat interface (partial)

---

## HTTP API Reference

### Gateway
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/health`, `/api/status` | Health check / gateway status |
| GET | `/api/gateway/pending` | List pending agents |
| POST | `/api/gateway/reload` | Reload gateway config |

### Agents
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/agents` | List all agents (full info: model, workspace, status, session_count) |
| POST | `/api/agents` | Create agent (persists to config + creates agent directory) |
| PUT | `/api/agents/:id` | Update agent (persists changes) |
| DELETE | `/api/agents/:id` | Delete agent (removes from config + runtime) |
| POST | `/api/agents/:id/message` | Send message to agent |

### Providers
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/providers` | List registered LLM providers |
| GET | `/api/providers/:id` | Get provider details |
| POST | `/api/providers/:id/test` | Test provider connectivity |
| POST | `/api/chat` | Route chat completion through gateway |
| GET | `/api/credentials/schemas` | Dynamic credential schemas from all plugins |

### Credentials
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/credentials/status` | Vault status |
| GET | `/api/credentials[/endpoints]` | List endpoints |
| POST | `/api/credentials[/endpoints]` | Create endpoint |
| DELETE | `/api/credentials[/endpoints]/:id` | Delete endpoint |
| POST | `/api/credentials/endpoints/:id/credential` | Store credential |
| POST | `/api/credentials/init` | Initialize vault |
| POST | `/api/credentials/unlock` | Unlock vault |
| POST | `/api/credentials/lock` | Lock vault |
| GET | `/api/credentials/permissions` | List permission rules |
| POST | `/api/credentials/permissions` | Add permission rule |
| DELETE | `/api/credentials/permissions/:id` | Remove permission rule |
| GET | `/api/credentials/audit` | View audit log (`?limit=N`) |
| GET | `/api/credentials/approvals` | List pending HIL approvals |
| POST | `/api/credentials/approvals/:id/approve` | Approve HIL request |
| POST | `/api/credentials/approvals/:id/deny` | Deny HIL request |
| POST | `/api/credentials/approvals/respond` | Generic approval response |

---

## What Needs Implementation

### High Priority

1. **Channel Manager** — unified multi-channel coordination, message routing, binding system
2. **Signal Channel** — real implementation (currently placeholder)
3. **Streaming Responses** — SSE/WebSocket streaming for chat UI
4. **Tool Sandboxing** — container or process-based sandbox for `exec` tool
5. **Credential Injection** — automatic credential injection into tool execution context

### Medium Priority

1. **Cron Scheduler** — job schema, schedule types, execution loop
2. **Skills System** — skill discovery, prompt injection, install specs
3. **Plugin Execution** — WASM or native plugin runtime
4. **Config Hot Reload** — live config updates without restart
5. **WebSocket Protocol** — full bidirectional communication

### Lower Priority

1. **Media Pipeline** — image/audio/video processing per channel
2. **Additional Channels** — Slack, WhatsApp, iMessage, Matrix, IRC
3. **Ollama Provider** — local model support
4. **Mobile Responsive Web UI**
5. **i18n / Theming**

---

## Known Issues

- `rockbot-credentials` has 17 pre-existing test failures in vault storage/manager tests
- `aws_smithy_types::Document` doesn't impl `Serialize` — manual converters in `bedrock.rs`
- Gateway uptime tracking returns 0 (TODO)
- Memory usage reporting returns 0 (TODO)
- SSH agent vault unlock not yet implemented

---

## Configuration Example

```toml
[gateway]
bind_host = "127.0.0.1"
port = 18080

[agents.defaults]
model = "anthropic/claude-sonnet-4-20250514"
workspace = "~/.config/rockbot/agents"

[[agents.list]]
id = "main"
model = "anthropic/claude-sonnet-4-20250514"

[[agents.list]]
id = "researcher"
model = "bedrock/anthropic.claude-3-5-sonnet-20241022-v2:0"
parent_id = "main"

[tools]
profile = "standard"

[security.sandbox]
mode = "tools"
scope = "session"

[credentials]
enabled = true
vault_path = "~/.config/rockbot/vault"
unlock_method = "env"
password_env_var = "RUSTCLAW_VAULT_PASSWORD"
default_permission = "deny"

[providers.anthropic]
auth_mode = "auto"

[providers.bedrock]
region = "us-east-1"
```

## Running

```bash
# Build
cargo build

# Run tests
cargo test --workspace --exclude rockbot-credentials

# Run gateway (foreground)
cargo run -- --config ~/.config/rockbot/config.toml gateway run

# TUI
cargo run -- --config ~/.config/rockbot/config.toml tui

# Credential management
cargo run -- credentials status
cargo run -- credentials add homeassistant -t home_assistant -u http://homeassistant:8123
cargo run -- credentials list
```
