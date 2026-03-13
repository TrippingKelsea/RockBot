# 🦀 RockBot

A Rust-native AI agent framework with secure credential management.

[![Build Status](https://github.com/TrippingKelsea/rockbot/workflows/CI/badge.svg)](https://github.com/TrippingKelsea/rockbot/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Overview

RockBot is a self-hosted multi-channel AI gateway and agent framework. It routes messages from Discord, Telegram, and other channels through a central gateway to AI agents backed by Anthropic, OpenAI, or AWS Bedrock. Credentials are stored in an encrypted vault and injected into tool execution at runtime, never exposed to the agent directly.

### Key Features

- **🔐 Secure Credential Vault** - AES-256-GCM encryption with Argon2id key derivation
- **👤 Human-in-the-Loop (HIL)** - Approval workflow for sensitive operations
- **📊 Terminal UI** - Full-featured TUI built with ratatui
- **🌐 Web Dashboard** - Browser-based management interface
- **🤖 Multi-Provider LLM** - Anthropic, OpenAI, AWS Bedrock (all working)
- **🔧 Agentic Tool Execution** - Dynamic iteration scaling, loop detection, semantic context compaction
- **🛠️ 9 Built-in Tools** - read, write, edit, exec, glob, grep, patch, memory_get, memory_search
- **📝 Audit Logging** - Hash-chained tamper-evident logs

## Quick Start

### Installation

```bash
# Clone the repository
git clone https://github.com/TrippingKelsea/rockbot.git
cd rockbot

# Build
cargo build --release

# Run
./target/release/rockbot --help
```

### First Run

```bash
# Initialize configuration
rockbot config init

# Start the gateway
rockbot gateway

# Or launch the TUI
rockbot tui
```

### Add a Credential

```bash
# Add Home Assistant endpoint
rockbot credentials add homeassistant \
  --type home_assistant \
  --url http://homeassistant.local:8123
```

## Documentation

- **[User Guide](docs/user-guide/)** - Installation, configuration, usage
- **[Architecture](docs/architecture/)** - System design and crate structure
- **[Feature Matrix](docs/FEATURES.md)** - Implementation status
- **[API Reference](#api-reference)** - Generated from source

## Crate Structure

| Crate | Description |
|-------|-------------|
| `rockbot` | Main binary entry point |
| `rockbot-cli` | CLI commands and TUI |
| `rockbot-core` | Gateway, agents, sessions, web UI |
| `rockbot-credentials` | Encrypted credential vault |
| `rockbot-credentials-schema` | Shared credential schema types |
| `rockbot-llm` | LLM provider abstraction (Anthropic, OpenAI, Bedrock) |
| `rockbot-memory` | Memory and search system |
| `rockbot-security` | Capability system and sandboxing |
| `rockbot-tools` | Built-in agent tools |
| `rockbot-tools-credentials` | Credential vault access tool |
| `rockbot-tools-mcp` | MCP server connection tool |
| `rockbot-tools-markdown` | Markdown processing tool |
| `rockbot-channels` | Channel traits and registry |
| `rockbot-channels-discord` | Discord channel (Serenity) |
| `rockbot-channels-telegram` | Telegram channel (Teloxide) |
| `rockbot-channels-signal` | Signal channel (placeholder) |
| `rockbot-plugins` | Plugin system |
| `rockbot` | Binary entry point |

See [Crate Structure](docs/architecture/crates.md) for details.

## Configuration

Configuration lives at `~/.config/rockbot/rockbot.toml`:

```toml
[gateway]
bind_host = "127.0.0.1"
port = 18080

[agents.defaults]
model = "anthropic/claude-sonnet-4-20250514"

[[agents.list]]
id = "main"

[credentials]
enabled = true
vault_path = "~/.local/share/rockbot/credentials"
```

See [Configuration Reference](docs/user-guide/configuration.md) for all options.

## Security Model

RockBot implements defense in depth:

1. **Encryption at Rest** - Credentials stored with AES-256-GCM
2. **Key Derivation** - Argon2id prevents brute-force attacks
3. **Capability System** - Tools can only access what's explicitly allowed
4. **HIL Approval** - Sensitive operations require human consent
5. **Audit Trail** - All credential access logged with hash chain

Credentials never cross the agent boundary. They're injected into tool execution and sanitized from responses.

```
Agent Request: "Turn on the lights"
    │
    ▼
Tool needs credential (saggyclaw://homeassistant/api/...)
    │
    ▼
Permission check: Allow / AllowHIL / Deny
    │
    ▼
If allowed: Inject credential, execute, sanitize response
```

See [Security Model](docs/architecture/security.md) for details.

## API Reference

Generate API documentation from source:

```bash
cargo doc --open --no-deps
```

### HTTP API Endpoints

The gateway exposes 30+ endpoints. Key ones are listed below; see [STATUS.md](STATUS.md) for the full reference.

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/health`, `/api/status` | Health check / gateway status |
| GET | `/api/agents` | List agents (model, status, session count) |
| POST | `/api/agents` | Create agent |
| PUT | `/api/agents/:id` | Update agent |
| DELETE | `/api/agents/:id` | Delete agent |
| POST | `/api/agents/:id/message` | Send message to agent |
| GET | `/api/providers` | List registered LLM providers |
| POST | `/api/providers/:id/test` | Test provider connectivity |
| POST | `/api/chat` | Route chat completion through gateway |
| GET | `/api/credentials/schemas` | Dynamic credential schemas from all plugins |
| GET | `/api/credentials/status` | Vault status |
| GET | `/api/credentials` | List credential endpoints |
| POST | `/api/credentials` | Create credential endpoint |
| POST | `/api/credentials/init` | Initialize vault |
| POST | `/api/credentials/unlock` | Unlock vault |
| POST | `/api/credentials/lock` | Lock vault |
| GET | `/api/credentials/permissions` | List permission rules |
| POST | `/api/credentials/permissions` | Add permission rule |
| GET | `/api/credentials/audit` | View audit log |
| GET | `/api/credentials/approvals` | Pending HIL approvals |
| POST | `/api/credentials/approvals/:id/approve` | Approve HIL request |
| POST | `/api/credentials/approvals/:id/deny` | Deny HIL request |

## Development

### Building

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run tests
cargo test --workspace

# Run specific crate tests
cargo test -p rockbot-credentials

# Run clippy
cargo clippy --workspace --all-features
```

### Project Status

See [STATUS.md](STATUS.md) for detailed implementation status and [FEATURES.md](docs/FEATURES.md) for the feature matrix.

**Current focus:**
- [ ] Streaming responses — wire existing LLM streaming through agent/gateway/UI
- [ ] Credential injection — automatic injection into tool execution
- [ ] Channel routing — binding system for channel→agent message flow
- [ ] Subagent delegation — parent agent spawning child agents
- [ ] WebSocket protocol — real-time UI updates

### Code Quality

Workspace-level Clippy lint configuration enforces code quality:
- `unwrap_used`, `expect_used`, `panic` — warned (tracked for elimination)
- Style lints — `redundant_closure`, `derivable_impls`, `uninlined_format_args`, etc.
- All original clippy warnings resolved; all tests passing

## Contributing

Contributions welcome! Please read [CONTRIBUTING.md](CONTRIBUTING.md) before submitting PRs.

## License

MIT License - see [LICENSE](LICENSE) for details.

## Acknowledgments

- Credential system ported from [SAGgyClaw](https://github.com/TrippingKelsea/saggyclaw)
- TUI built with [ratatui](https://github.com/ratatui-org/ratatui)
- Inspired by [OpenClaw](https://github.com/openclaw/openclaw)
