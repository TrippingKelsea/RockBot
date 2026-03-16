# RockBot Feature Matrix

This document tracks feature implementation status and helps identify gaps between planned functionality and current implementation.

**Legend:**
- ✅ Implemented and tested
- 🚧 Partially implemented / in progress
- 📋 Planned / designed but not started
- ❌ Not planned for MVP

---

## Core Framework (`rockbot-core`)

### Gateway Server

| Feature | Status | Notes |
|---------|--------|-------|
| HTTP server (hyper) | ✅ | Async, production-ready |
| Health check endpoint | ✅ | `GET /health` |
| Agent listing | ✅ | `GET /api/agents` |
| Agent messaging | ✅ | `POST /api/agents/{id}/message` |
| Agent CRUD (create/update/delete) | ✅ | `POST/PUT/DELETE /api/agents` |
| WebSocket support | ✅ | Full duplex streaming, health checks, remote exec |
| TLS/HTTPS | ✅ | Self-signed bootstrap or PKI-managed certs |
| Mutual TLS (mTLS) | ✅ | Optional or mandatory client cert verification |
| Rate limiting | 📋 | |
| Authentication | 📋 | `require_api_key` field exists, not enforced |

### Configuration

| Feature | Status | Notes |
|---------|--------|-------|
| TOML-based config | ✅ | |
| Environment variable expansion | ✅ | `${VAR}` syntax |
| Hot-reload via file watcher | ✅ | notify crate |
| Config validation | ✅ | |
| Config migration | 📋 | |

### Session Management

| Feature | Status | Notes |
|---------|--------|-------|
| SQLite persistence | ✅ | |
| Message history | ✅ | |
| Token usage tracking | ✅ | |
| Session CRUD | ✅ | |
| Session archival | 🚧 | CLI command exists, partial implementation |
| Session export | 📋 | JSON/Markdown export |

### Agent Engine

| Feature | Status | Notes |
|---------|--------|-------|
| Message processing pipeline | ✅ | |
| Tool execution loop | ✅ | 32-160 dynamic iterations |
| Tool loop detection | ✅ | warn/critical/circuit breaker levels |
| Context management | ✅ | |
| Semantic context compaction | ✅ | LLM-based |
| Continuation nudges | ✅ | 3-level escalation |
| `<think>` reasoning block support | ✅ | |
| Streaming responses | 📋 | Infrastructure exists in LLM layer, not wired through agent/gateway |
| Multi-turn conversation | ✅ | |
| Temperature/max_tokens per agent | ✅ | Configurable per agent |

---

## Credential Management (`rockbot-credentials`)

### Vault

| Feature | Status | Notes |
|---------|--------|-------|
| AES-256-GCM encryption | ✅ | |
| Argon2id key derivation | ✅ | |
| Password unlock | ✅ | |
| Keyfile unlock | ✅ | |
| Age encryption | 🚧 | Stubbed |
| SSH key unlock | 🚧 | Stubbed |
| Auto-unlock via env var | ✅ | `ROCKBOT_VAULT_PASSWORD` |
| redb storage backend | ✅ | Replaced flat JSON files; auto-migration of legacy data |
| Generic KV store | ✅ | Namespaced key-value storage in vault |
| ChaCha20 storage encryption | ✅ | Block-level encryption via `redb::StorageBackend` |
| OpenRaft replication | 🚧 | Feature-gated (`replication`); log store + state machine implemented, network stubs |

### Endpoint Types

| Type | Status | Notes |
|------|--------|-------|
| Home Assistant | ✅ | Long-lived access token |
| Generic REST API | ✅ | Bearer token |
| OAuth2 Service | 🚧 | Token storage works, automated flow not implemented |
| API Key Service | ✅ | Custom header support |
| Basic Auth | ✅ | Username/password |
| Bearer Token | ✅ | Generic bearer |

### Permission System

| Feature | Status | Notes |
|---------|--------|-------|
| Allow | ✅ | Immediate grant |
| AllowHIL (Human-in-Loop) | ✅ | Approval queue |
| AllowHIL2FA | 📋 | YubiKey integration |
| Deny | ✅ | |
| Glob pattern matching | ✅ | `saggyclaw://api/**` |
| Persistent rules | ✅ | Stored in redb PERMISSIONS table |

### Audit Logging

| Feature | Status | Notes |
|---------|--------|-------|
| Hash-chained log | ✅ | Tamper-evident |
| Operation tracking | ✅ | CRUD + permission changes |
| Verification | ✅ | CLI command |
| Log rotation | 📋 | |
| Log export | 📋 | |

### HTTP API

| Endpoint | Status | Notes |
|----------|--------|-------|
| `GET /api/credentials/status` | ✅ | |
| `GET /api/credentials` | ✅ | List endpoints |
| `POST /api/credentials` | ✅ | Create endpoint |
| `DELETE /api/credentials/:id` | ✅ | |
| `POST .../credential` | ✅ | Store secret |
| `GET /api/credentials/permissions` | ✅ | |
| `POST /api/credentials/permissions` | ✅ | |
| `DELETE /api/credentials/permissions/:id` | ✅ | |
| `GET /api/credentials/audit` | ✅ | |
| `GET /api/credentials/approvals` | ✅ | |
| `POST /api/credentials/approvals/:id/approve` | ✅ | |
| `POST /api/credentials/approvals/:id/deny` | ✅ | |
| `POST /api/credentials/unlock` | ✅ | |
| `POST /api/credentials/lock` | ✅ | |

---

## LLM Providers (`rockbot-llm`)

| Provider | Status | Notes |
|----------|--------|-------|
| Mock provider | ✅ | For testing |
| Anthropic Claude | ✅ | Via Claude Code SDK OAuth |
| OpenAI | ✅ | |
| AWS Bedrock | ✅ | Converse API |
| Streaming support | ✅ | All 3 providers implement `stream_completion` |
| Retry/backoff | ✅ | Exponential with jitter |
| Ollama (local) | 📋 | |

---

## Tools (`rockbot-tools`)

### Built-in Tools

| Tool | Status | Notes |
|------|--------|-------|
| `read` | ✅ | File reading with offset/limit |
| `write` | ✅ | File writing |
| `edit` | ✅ | Text editing |
| `exec` | ✅ | Shell execution |
| `glob` | ✅ | File pattern matching |
| `grep` | ✅ | Content searching |
| `patch` | ✅ | Diff application |
| `memory_get` | ✅ | Full profile |
| `memory_search` | ✅ | Full profile |
| `web_search` | 📋 | |
| `web_fetch` | 📋 | |
| `browser` | 📋 | |

### Tool System

| Feature | Status | Notes |
|---------|--------|-------|
| Tool registry | ✅ | |
| Profile-based loading | ✅ | minimal/standard/full |
| Capability-based filtering | ✅ | |
| JSON Schema generation | ✅ | Tools provide schemas for LLM function calling |
| Tool result types | ✅ | |

### Tool Provider Crates

| Crate | Status | Notes |
|-------|--------|-------|
| rockbot-tools-credentials | ✅ | Vault access tool |
| rockbot-tools-mcp | ✅ | MCP server connection |
| rockbot-tools-markdown | ✅ | Markdown processing |

---

## PKI and mTLS (`rockbot-pki`)

| Feature | Status | Notes |
|---------|--------|-------|
| CA generation (self-signed) | ✅ | `rockbot cert ca generate` |
| Client certificate issuance | ✅ | Gateway, Agent, TUI roles with EKU |
| CSR signing | ✅ | Local and remote (via gateway API) |
| Certificate revocation + CRL | ✅ | `rockbot cert client revoke` |
| Certificate rotation | ✅ | Revoke + reissue |
| PKI index (JSON registry) | ✅ | `index.json` tracks all certs |
| Enrollment tokens | ✅ | One-time/limited-use, optional expiry |
| Gateway mTLS enforcement | ✅ | `WebPkiClientVerifier`, mandatory or optional |
| PSK enrollment endpoint | ✅ | `POST /api/cert/sign` |
| Config patching (`cert install`) | ✅ | Writes TLS paths into `rockbot.toml` |
| `KeyBackend` trait | ✅ | `FileBackend` implemented |
| Hardware key backends (PKCS#11, YubiKey) | 📋 | Trait stubbed, `KeyHandle::Hardware` variant |
| Client-side cert loading (TUI/agent) | 📋 | TUI currently accepts self-signed |
| OCSP stapling | 📋 | |
| Automatic cert renewal | 📋 | |
| S3 CA distribution | ✅ | `rockbot cert ca publish`, `bedrock-deploy` feature |
| Route53 DNS auto-provisioning | ✅ | Private hosted zone, CNAME records |
| AWS credential auto-import | ✅ | Env/shared-credentials discovery, vault storage |

---

## S3 Deploy (`rockbot-deploy`)

| Feature | Status | Notes |
|---------|--------|-------|
| S3 bucket auto-creation | ✅ | `auto_create_bucket` config, us-east-1 quirk handled |
| CA cert upload to S3 | ✅ | `application/x-pem-file` content type |
| Public bucket policy | ✅ | Best-effort, warns on Block Public Access |
| Route53 private hosted zone | ✅ | Auto-created if missing, idempotent |
| CNAME registration (cluster) | ✅ | UUID + optional friendly name |
| CNAME registration (client) | ✅ | Per-client UUID records |
| Custom S3 endpoint | ✅ | Compile-time + runtime override, path-style |
| AWS credential auto-import | ✅ | Env vars, shared credentials file, vault KV |
| Gateway startup provisioning | ✅ | `upload_on_startup` config flag |
| CLI interactive publish | ✅ | `rockbot cert ca publish` with conflict resolution |

---

## Security (`rockbot-security`)

| Feature | Status | Notes |
|---------|--------|-------|
| Capability enum | ✅ | FilesystemRead/Write, ProcessExecute, etc. |
| Security context | ✅ | Session-scoped |
| Capability checking | ✅ | |
| Sandbox (container) | 📋 | |
| Sandbox (process) | 📋 | |
| Path canonicalization | 📋 | |
| Command allowlisting | 📋 | |

---

## Memory (`rockbot-memory`)

| Feature | Status | Notes |
|---------|--------|-------|
| Document loading | ✅ | |
| Keyword search | ✅ | |
| Core memory (JSON) | ✅ | |
| Vector index | 🚧 | TF-IDF based, being implemented |
| Semantic search | 🚧 | TF-IDF cosine similarity, being implemented |
| Memory compaction | 📋 | |

---

## CLI (`rockbot-cli`)

### Commands

| Command | Status | Notes |
|---------|--------|-------|
| `gateway` | ✅ | Start gateway server |
| `config show` | ✅ | |
| `config validate` | ✅ | |
| `config init` | ✅ | |
| `session list` | ✅ | |
| `session show` | ✅ | |
| `session history` | ✅ | |
| `session archive` | 🚧 | |
| `session delete` | ✅ | |
| `agent list` | ✅ | |
| `agent status` | ✅ | |
| `agent message` | ✅ | |
| `agent create` | 🚧 | |
| `tool list` | ✅ | |
| `tool info` | ✅ | |
| `tool test` | 🚧 | |
| `cert ca generate/info/rotate` | ✅ | CA lifecycle |
| `cert client generate/list/info/revoke/rotate` | ✅ | Client cert management |
| `cert sign` | ✅ | Offline CSR signing |
| `cert install` | ✅ | Patch config with cert paths |
| `cert verify` | ✅ | Cert/key match + chain |
| `cert info` | ✅ | PEM inspection |
| `cert enroll create/list/revoke/submit` | ✅ | Remote enrollment |
| `credentials status` | ✅ | |
| `credentials list` | ✅ | |
| `credentials add` | ✅ | |
| `credentials remove` | ✅ | |
| `credentials unlock` | ✅ | |
| `credentials lock` | ✅ | |
| `credentials permissions` | ✅ | |
| `credentials audit` | ✅ | |
| `doctor` | 🚧 | |
| `migrate` | 📋 | |

### Post-Build Testing

| Feature | Status | Notes |
|---------|--------|-------|
| strace-based perf tests | ✅ | No Tokio runtime on help paths, syscall budget, startup time |
| strace-based security tests | ✅ | No network, no sensitive reads, no child execs on info paths |
| Binary size budget | ✅ | Release binary under 150MB |
| Binary permissions check | ✅ | No world-writable bit |
| CI integration | ✅ | Runs after build in CI pipeline |

### TUI (Terminal UI)

| Feature | Status | Notes |
|---------|--------|-------|
| Async event loop | ✅ | tokio::select! |
| Dashboard view | ✅ | Card strip layout |
| Credentials view (4 sub-tabs) | ✅ | All, Model, Communication, Tool |
| Agents view | ✅ | CRUD, modal editing |
| Sessions view | ✅ | Card strip + chat |
| Models view | ✅ | Dynamic provider list, test |
| Settings view | 🚧 | |
| Vault unlock modal | ✅ | Auto-unlock for keyfile |
| Real data binding | ✅ | Gateway API calls wired |
| Gateway API calls | ✅ | |
| Rounded borders | ✅ | `BorderType::Rounded` on all blocks |
| Scrollbar widgets | ✅ | Sessions chat, credentials endpoints, model list |
| tachyonfx effects | ✅ | Modal coalesce/dissolve, page fade, background dim |
| Floating top bar | ✅ | `[tui] floating_bar = true` (default), content scrolls behind |
| Context menu | ✅ | `?` key opens page-specific action menu |
| Flex layout | ✅ | `Constraint::Fill(1)` + `Flex::Start` card strips |
| TUI config (`[tui]`) | ✅ | `floating_bar`, `animations` toggles |

---

## Web UI (`rockbot-core::web_ui`)

| Feature | Status | Notes |
|---------|--------|-------|
| Embedded HTML SPA | ✅ | Vanilla JS, no framework (~1645 lines) |
| Dashboard | ✅ | |
| Credentials page | ✅ | 4 sub-tabs, schema-driven |
| Agents page | ✅ | CRUD, subagents |
| Sessions page | ✅ | Chat |
| Models page | ✅ | Test, configure |
| Settings page | 🚧 | |
| Real-time updates | 📋 | WebSocket needed |

---

## Channels (`rockbot-channels`)

| Channel | Status | Notes |
|---------|--------|-------|
| Channel trait + registry | ✅ | |
| Discord | ✅ | Serenity: connect, send, events, embeds |
| Telegram | ✅ | Teloxide |
| Signal | 📋 | Placeholder only |
| Slack | 📋 | |
| IRC | 📋 | |

---

## Config Diagnostics (`rockbot-doctor`)

Requires the `doctor-ai` feature flag. Reuses `rockbot-overseer`'s candle/GGUF
inference stack to run a small local model for config analysis.

| Feature | Status | Notes |
|---------|--------|-------|
| AI config error diagnosis | ✅ | Human-readable explanation of TOML parse/validation errors |
| Auto-repair of TOML config | ✅ | Structure-preserving edits via `toml_edit` |
| Migration detection | ✅ | Detects and rewrites deprecated/renamed fields |
| Startup interception | ✅ | Runs automatically on config load failure |
| `rockbot doctor` CLI command | 🚧 | Command exists; AI path wired, repair confirmation UI pending |
| Self-learning fix recall | ✅ | SHA-256 fingerprinted fixes stored in JSONL, instant recall on repeat errors |
| Few-shot prompt injection | ✅ | Recent successful fixes used as examples for the model |
| Fix verification loop | ✅ | Patched TOML re-parsed before committing; reverts on failure |

---

## Plugins (`rockbot-plugins`)

| Feature | Status | Notes |
|---------|--------|-------|
| Plugin trait | ✅ | |
| Plugin registry | 🚧 | Scaffold only |
| WASM runtime | 📋 | |
| Plugin discovery | 📋 | |
| Plugin isolation | 📋 | |

---

## Feature Profiles

Meta feature flags for the binary crate (`rockbot`). Each profile is additive.

| Profile | Includes | Use Case |
|---------|----------|----------|
| `conservative` (default) | bedrock, telegram, signal, tools-credentials, tools-mcp, tools-markdown | Production — stable, minimal dependencies |
| `enhanced` | conservative + overseer, doctor-ai, vault-replication | Production+ — AI oversight, config diagnostics, HA vault |
| `experimental` | enhanced + otel, bedrock-deploy | Development/staging — telemetry, cloud provisioning |
| `enshitify` | discord | Discord channel support |

```bash
# Default build (conservative)
cargo build --release

# Enhanced build
cargo build --release --features enhanced --no-default-features

# Experimental (everything)
cargo build --release --features experimental --no-default-features

# Cherry-pick: conservative + one extra
cargo build --release --features "conservative,otel"
```

---

## Gap Analysis Summary

### Critical Path Items

1. **Streaming responses** - LLM streaming is implemented in all three providers but not wired through the agent engine, gateway API, or UI layers.
2. **Credential injection** - Tools execute but have no mechanism to retrieve vault credentials from the execution context at call time.
3. **Routing/binding system** - No channel-to-agent message routing exists; channels and agents run independently with no dispatch layer connecting them.
4. **Subagent delegation** - Agent-to-agent task delegation is not implemented; parent/child relationship fields exist in the data model only.
5. **WebSocket protocol** - Required for real-time UI updates in both TUI and Web UI; placeholder exists in gateway but handler is not implemented.

### Nice to Have (Post-MVP)

1. Additional LLM providers (Ollama)
2. Signal channel integration
3. WASM plugin system
4. Sandbox implementation (container and process)
5. Session export (JSON/Markdown)

### Technical Debt

1. OAuth2 automated flow not implemented (token storage works, but acquisition is manual)
2. Age encryption and SSH key unlock are stubbed in the vault
3. Test coverage gaps in some modules
