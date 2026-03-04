# 🦀 Krabbykrus

A Rust-native AI agent framework with secure credential management.

[![Build Status](https://github.com/TrippingKelsea/krabbykrus/workflows/CI/badge.svg)](https://github.com/TrippingKelsea/krabbykrus/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Overview

Krabbykrus is a local-first AI agent framework that prioritizes security and credential safety. Credentials never cross the agent boundary—they're stored in an encrypted vault and injected into tool execution at runtime.

### Key Features

- **🔐 Secure Credential Vault** - AES-256-GCM encryption with Argon2id key derivation
- **👤 Human-in-the-Loop (HIL)** - Approval workflow for sensitive operations  
- **📊 Terminal UI** - Full-featured TUI built with ratatui
- **🌐 Web Dashboard** - Browser-based management interface
- **🤖 Multi-Provider LLM** - Anthropic, OpenAI, Ollama, Bedrock (planned)
- **🔧 Extensible Tools** - Plugin architecture for custom capabilities
- **📝 Audit Logging** - Hash-chained tamper-evident logs

## Quick Start

### Installation

```bash
# Clone the repository
git clone https://github.com/TrippingKelsea/krabbykrus.git
cd krabbykrus

# Build
cargo build --release

# Run
./target/release/krabbykrus --help
```

### First Run

```bash
# Initialize configuration
krabbykrus config init

# Start the gateway
krabbykrus gateway

# Or launch the TUI
krabbykrus tui
```

### Add a Credential

```bash
# Add Home Assistant endpoint
krabbykrus credentials add homeassistant \
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
| `krabbykrus` | Main binary entry point |
| `krabbykrus-cli` | CLI commands and TUI |
| `krabbykrus-core` | Gateway, agents, sessions, web UI |
| `krabbykrus-credentials` | Encrypted credential vault |
| `krabbykrus-llm` | LLM provider abstraction |
| `krabbykrus-memory` | Memory and search system |
| `krabbykrus-security` | Capability system and sandboxing |
| `krabbykrus-tools` | Built-in agent tools |
| `krabbykrus-channels` | Communication channels |
| `krabbykrus-plugins` | Plugin system |

See [Crate Structure](docs/architecture/crates.md) for details.

## Configuration

Configuration lives at `~/.config/krabbykrus/krabbykrus.toml`:

```toml
[gateway]
bind_host = "127.0.0.1"
port = 8765

[agents.defaults]
model = "anthropic/claude-sonnet-4-20250514"

[[agents.list]]
id = "main"

[credentials]
enabled = true
vault_path = "~/.local/share/krabbykrus/credentials"
```

See [Configuration Reference](docs/user-guide/configuration.md) for all options.

## Security Model

Krabbykrus implements defense in depth:

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

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | Health check |
| `/api/agents` | GET | List agents |
| `/api/agents/{id}/message` | POST | Send message to agent |
| `/api/credentials` | GET | List credential endpoints |
| `/api/credentials` | POST | Create endpoint |
| `/api/credentials/status` | GET | Vault status |
| `/api/credentials/unlock` | POST | Unlock vault |
| `/api/credentials/approvals` | GET | Pending HIL approvals |

See [STATUS.md](STATUS.md) for complete API reference.

## Development

### Building

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run tests
cargo test

# Run specific crate tests
cargo test -p krabbykrus-credentials
```

### Project Status

See [STATUS.md](STATUS.md) for detailed implementation status and [FEATURES.md](docs/FEATURES.md) for the feature matrix.

**Current focus:**
- [ ] Real LLM provider (Anthropic)
- [ ] Built-in tool implementations
- [ ] TUI/Web UI real data binding
- [ ] Channel integrations

## Contributing

Contributions welcome! Please read [CONTRIBUTING.md](CONTRIBUTING.md) before submitting PRs.

## License

MIT License - see [LICENSE](LICENSE) for details.

## Acknowledgments

- Credential system ported from [SAGgyClaw](https://github.com/TrippingKelsea/saggyclaw)
- TUI built with [ratatui](https://github.com/ratatui-org/ratatui)
- Inspired by [OpenClaw](https://github.com/openclaw/openclaw)
