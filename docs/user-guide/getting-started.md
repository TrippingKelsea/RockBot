# Getting Started

## Prerequisites

- **Rust 1.75+** — install via [rustup](https://rustup.rs/)

## Installation

```bash
git clone https://github.com/TrippingKelsea/rockbot.git
cd rockbot
make && make install
```

The binary is installed to `~/.local/bin/rockbot`.

Ensure `~/.local/bin` is in your `$PATH`


# Shell Completion
You can also generate shell completions directly from the CLI:

```bash
./rockbot completion zsh
./rockbot completion bash
./rockbot completion fish
```

# Diagnostics
```bash
rockbot --version
rockbot doctor        # diagnostic checks
```

## Initial Setup

### Generate Gateway Config

```bash
rockbot config init gateway
# Creates ~/.config/rockbot/rockbot.toml
```

This creates a bootstrap-only gateway config and a self-signed TLS
certificate. Runtime entities such as agents should be created through the
gateway or TUI and persisted in the runtime storage layer, not declared in
`rockbot.toml`.

### Minimal Configuration

```toml
# ~/.config/rockbot/rockbot.toml

[gateway]
bind_host = "0.0.0.0"
port = 18181
client_port = 18182

[gateway.public]
serve_webapp = true
serve_ca = true
enrollment_enabled = true

[client]
gateway_host = "127.0.0.1"
https_port = 18181
client_port = 18182
```


### Mutual TLS (mTLS)

RockBot uses mTLS for client/server authentication and authorization.

#### Set Up the CA and Certificates

```bash
# If you are running the repo-local build, use `./rockbot` instead of `rockbot`.

# Initialize a Certificate Authority (valid 10 years)
rockbot cert ca generate --days 3650

# Generate a gateway certificate
rockbot cert client generate --name gateway --role gateway \
  --san localhost --san 127.0.0.1 --days 365

# Install into rockbot.toml (writes [pki] and enables gateway mTLS policy)
rockbot cert install --name gateway

# Initialize the credential store
rockbot credentials init
```

### Start the Gateway

```bash
rockbot gateway run
# INFO Gateway public listener on 0.0.0.0:18181 (TLS)
# INFO Gateway client listener on 0.0.0.0:18182 (TLS/mTLS)
```

### Connect with the TUI

#### Localhost

From the same machine:
```bash
rockbot tui
```

### Enroll a Remote Client

Enrollment happens over the public HTTPS listener, so you do not need to
temporarily disable client-certificate enforcement for the client listener. If
you do not want browser/bootstrap enrollment exposed, set:

```toml
[gateway.public]
enrollment_enabled = false
```

From the gateway machine:
```bash
# Generate a TUI client certificate from the CA
rockbot cert enroll create --name my-tui --role tui --days 365

# Tokens can embed multiple auth roles:
rockbot cert enroll create --name my-tui-agent --role client --role tui --role agent --uses 1 --expires 24h
# Output: Token: <uuid>
```

From another machine on the network:
```bash
rockbot config init client --gateway-ip (gateway-ip)

# On the remote client: enroll with the gateway
rockbot cert enroll submit --name my-tui --psk (token) --ca-fingerprint (sha256-fingerprint) 

# Install into the client's config
rockbot cert install --name my-tui

rockbot tui
```

The client bootstrap config points the TUI at the dedicated client listener.
You can still override it with `-g host:port` when needed.

Native clients use the client-listener WebSocket for both chat traffic and
gateway control-plane requests such as provider, agent, and session management.
The public HTTPS listener is intentionally minimal: a Leptos-rendered browser
bootstrap shell, `/static/*`, health, CA publication, and optional enrollment.

### Agent Persistence Model

RockBot uses three different persistence layers during normal operation:

- `rockbot.toml` stores bootstrap settings, listener policy, provider config,
  defaults, and TUI preferences.
- `rockbot.data` stores shared control-plane state such as the agent registry,
  sessions, cron jobs, routing, and topology metadata.
- `agents/{agent_id}.data` stores the canonical per-agent vdisk for that
  agent's local documents and state.

When you create an agent from the TUI or `POST /api/agents`, RockBot creates a
registry/topology entry and initializes the per-agent vdisk with documents such
as `SOUL.md`, `SYSTEM-PROMPT.md`, `AGENTS.md`, and `MEMORY.md`.

### Open the Web UI Bootstrap

Navigate to `https://localhost:18181` in your browser and accept the
self-signed certificate when prompted. The browser app is now a bootstrap shell
served from the public listener. It exposes only:

- `/`
- `/static/*`
- `/health`
- `/api/cert/ca`
- `/api/cert/sign` when `gateway.public.enrollment_enabled = true`

The page lets you import a client certificate/key bundle into browser storage
and authenticate to the browser WebSocket control plane without exposing the
full REST management surface publicly.


### View and Manage Certificates

```bash
rockbot cert client list           # list all issued certs
rockbot cert client info --name X  # details for one cert
rockbot cert client revoke --name X  # revoke (regenerates CRL)
rockbot cert client rotate --name X --days 365 --backup  # rotate
rockbot cert ca info               # CA details
```

See `docs/architecture/pki.md` for the full PKI reference.

## Remote Tool Execution

Build with the `remote-exec` (default) feature to let the gateway dispatch tool calls
(file reads, shell commands) to your local machine:

```bash
cargo build --release -F remote-exec
```

If you only want the Noise transport primitives without remote executor
dispatch, build with:

```bash
cargo build --release -F noise
```

When the TUI connects, it automatically registers as a remote executor after
the Noise handshake completes. The Dashboard also exposes Noise and execution
target cards so you can verify registration state and choose whether tools run
on the active client, the gateway, or another connected executor.

## Setting Up Credentials

### Initialize the Vault

```bash
rockbot credentials init
```

### Add an Endpoint

```bash
rockbot credentials add homeassistant \
  --type home_assistant \
  --url http://homeassistant.local:8123
# You'll be prompted for the access token
```

### List Endpoints

```bash
rockbot credentials list
```

## Feature Flags

| Flag | Description |
|------|-------------|
| `conservative` | Default profile: bedrock + telegram + signal + built-in tool crates |
| `enhanced` | Conservative plus overseer, doctor-ai, and vault replication |
| `experimental` | Enhanced plus telemetry and S3/Route53 deployment helpers |
| `noise` | Noise handshake and transport primitives |
| `remote-exec` | Remote tool dispatch built on Noise |
| `overseer` | Embedded local-model agent oversight |
| `doctor-ai` | Local AI-powered configuration diagnostics and repair |
| `bedrock-deploy` | S3 CA distribution and Route53 DNS provisioning |
| `otel` | OpenTelemetry export |
| `http-insecure` | Allow plain HTTP (TLS is default) |
| `anthropic` | Anthropic API provider |
| `openai` | OpenAI API provider |
| `ollama` | Ollama local models |
| `all-providers` | Enable Anthropic, OpenAI, Ollama, and Bedrock together |
| `all-channels` | Enable Discord, Telegram, and Signal together |
| `all-tools` | Enable all built-in tool provider crates together |

## Troubleshooting

**Gateway won't start:**
```bash
# Check if port is in use
ss -tlnp | grep 18080
```

**TLS certificate issues:**
```bash
# Regenerate self-signed certificate (quick bootstrap)
rockbot config init --force

# Or inspect and verify existing certs
rockbot cert info --cert ~/.config/rockbot/pki/certs/gateway.crt
rockbot cert verify --cert gateway.crt --key gateway.key --ca ca.crt

# Rotate an expiring certificate
rockbot cert client rotate --name gateway --san localhost --days 365 --backup
```

**Vault won't unlock:**
```bash
# If you forgot your password or lost the vault key, you'll need to recreate the vault
rm -rf ~/.local/share/rockbot/credentials
rockbot credentials init
```

**Configuration errors:**
```bash
rockbot config validate
rockbot config show
```
