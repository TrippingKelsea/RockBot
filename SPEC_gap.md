# rockbot Implementation Gap Analysis

**Generated:** 2026-03-13
**Based on:** SPEC.md v1.0
**Codebase:** 18 crates, ~35,000 LOC

---

## Executive Summary

rockbot is approximately **40-45% complete** relative to the full SPEC.md specification. The credential vault and agent execution system are the most mature components (~85% complete each), followed by the TUI (~80%) and Web UI (~70%). All three LLM providers (Anthropic, OpenAI, Bedrock) are working with streaming support at the LLM layer. The biggest remaining gaps are streaming wired through the full agent/gateway/UI pipeline, the channel routing/binding system, the cron/scheduling system, and the subagent delegation mechanism.

---

## Implementation Status by Section

### Substantially Complete (>70%)

| Section | Status | Notes |
|---------|--------|-------|
| **14. Security Model** | ~85% | Credential vault, permissions, audit logging, HIL approval all implemented |
| **6. Session Management** | ~80% | SQLite persistence, session CRUD, message history, per-session chat state |
| **7. Agent System** | ~85% | Multi-tool execution (32-160 dynamic iterations), tool loop detection, semantic compaction, continuation nudges, `<think>` blocks, system prompt assembly, configurable temperature/max_tokens |
| **20. TUI** | ~80% | All 6 sections with real gateway data binding, agent CRUD modals, per-session chat with tool call display |
| **20. Web UI** | ~70% | All 6 sections at parity with TUI, cyberpunk theme, schema-driven provider config |
| **3. Gateway Protocol** | ~60% | HTTP API with 30+ endpoints; WebSocket upgrade placeholder only |

### Partially Implemented (30-70%)

| Section | Status | Notes |
|---------|--------|-------|
| **2. System Architecture** | ~50% | Gateway + Session Store + Agent engine working; Channel Manager, Routing Engine partial |
| **8. Tool System** | ~70% | 9 built-in tools (read, write, edit, exec, glob, grep, patch, memory_get, memory_search) + 3 plugin tools; missing web_search, web_fetch, browser |
| **10. Configuration** | ~55% | Config loading works, hot-reload watcher exists, temperature/max_tokens per agent; missing hot apply |
| **11. CLI Interface** | ~50% | gateway, credentials (full suite), session, agent, tool, config, doctor commands; missing models, cron, plugins, message |
| **15. Data Storage** | ~50% | Session SQLite works; transcript JSONL partial |
| **18-19. Error/Observability** | ~65% | Structured error handling with retry logic, error categorization, exponential backoff with jitter |
| **4. Channel System** | ~30% | Discord implemented (Serenity), Telegram implemented (Teloxide), Signal placeholder |

### Minimal/Stubbed (<30%)

| Section | Status | Notes |
|---------|--------|-------|
| **5. Routing System** | ~10% | No binding resolution, no route matching, no session key formatting |
| **9. Plugin System** | ~10% | Manifest loading only; no WASM, no hook execution |
| **13. Cron System** | ~5% | Heartbeat config field exists; no scheduler, no job execution |
| **16. Skills System** | ~0% | Not implemented |
| **17. Media Pipeline** | ~0% | Not implemented |
| **12. Message Context** | ~25% | Basic message struct with tool_call_id, metadata.extra for tool calls; MsgContext not fully populated |

---

## Detailed Gap Analysis

### 1. Gateway Protocol (Section 3)

#### Implemented
- [x] HTTP API server on configurable port (30+ endpoints)
- [x] JSON request/response
- [x] Auth token validation
- [x] Agent CRUD endpoints (`GET/POST/PUT/DELETE /api/agents`)
- [x] Session CRUD endpoints
- [x] Credential endpoints
- [x] Provider schema endpoints
- [x] WebSocket upgrade path (placeholder, not functional)

#### Missing
- [ ] Protocol version negotiation
- [ ] WebSocket event broadcasting with sequence numbers
- [ ] State version synchronization
- [ ] Full RPC method coverage:
  - [ ] `agent.invoke`, `agent.wait`, `agent.wake`, `agent.identity`
  - [ ] `chat.send`, `chat.abort`, `chat.inject`, `chat.history`
  - [ ] `sessions.compact`, `sessions.usage`
  - [ ] `channels.status`, `channels.send`, `channels.poll`
  - [ ] `config.set`, `config.apply`, `config.schema`
  - [ ] `cron.*` methods
- [ ] Reconnection handling
- [ ] Heartbeat/ping-pong

---

### 2. Channel System (Section 4)

#### Implemented
- [x] `ChannelPlugin` trait definition
- [x] Discord channel (Serenity-based)
  - [x] Connect/disconnect
  - [x] Send/edit/delete messages
  - [x] Event stream
  - [x] Embeds support
  - [ ] Components/buttons
  - [ ] Threads
- [x] Telegram channel (Teloxide-based)
  - [x] Connect/disconnect
  - [x] Send/receive messages
- [ ] Signal — placeholder only

#### Missing Channels
| Channel | Library | Priority |
|---------|---------|----------|
| **Signal** | signal-cli | High |
| **Slack** | Bolt | Medium |
| **WhatsApp** | Baileys | Medium |
| **iMessage** | BlueBubbles | Medium |
| Matrix | matrix-sdk | Low |
| IRC | irc-rust | Low |
| Google Chat | API | Low |
| MS Teams | Graph API | Low |

#### Missing Infrastructure
- [ ] Channel Manager (multi-channel coordination)
- [ ] Outbound delivery abstraction
- [ ] Text chunking per channel limits
- [ ] Media handling per channel
- [ ] Unified event normalization

---

### 3. Routing System (Section 5)

#### Implemented
- [x] Basic session key generation
- [x] Agent lookup by ID

#### Missing
- [ ] Binding system
  - [ ] Peer bindings
  - [ ] Guild bindings (Discord)
  - [ ] Account bindings
  - [ ] Channel bindings
- [ ] Route resolution priority chain
- [ ] Session key format parsing (`{scope}:{channel}:{identifier}`)
- [ ] Session scoping modes (per-sender, global, per-peer, etc.)
- [ ] Binding persistence and hot-update

---

### 4. Agent System (Section 7)

#### Implemented
- [x] Agent struct with config (model, workspace, parent_id, temperature, max_tokens)
- [x] LLM invocation — all three providers working (Anthropic, OpenAI, Bedrock)
- [x] Multi-tool execution loop (32-160 dynamic iterations)
- [x] Tool loop detection and circuit breaking
- [x] Semantic compaction of long conversations
- [x] Continuation nudges
- [x] `<think>` block handling
- [x] System prompt assembly (SOUL.md, SYSTEM-PROMPT.md injection)
- [x] Session transcript persistence
- [x] Agent directory creation (`~/.config/rockbot/agents/{id}/`)
- [x] Agent CRUD via gateway API with file persistence
- [x] Configurable temperature and max_tokens per agent

#### Missing
- [ ] **Streaming wired through agent/gateway/UI** — LLM layer has streaming, not connected end-to-end
- [ ] Thinking levels (off/minimal/low/medium/high)
- [ ] Model failover chain
- [ ] Rate limit detection and backoff
- [ ] Auth profile fallback
- [ ] Subagent delegation mechanism
- [ ] Abort handling mid-execution
- [ ] Response chunking for channel delivery limits

---

### 5. Tool System (Section 8)

#### Implemented
| Tool | Status | Notes |
|------|--------|-------|
| `read` | Complete | File reading with offset/limit |
| `write` | Complete | File writing |
| `edit` | Complete | File editing with diff |
| `exec` | Complete | Shell execution |
| `glob` | Complete | File pattern matching |
| `grep` | Complete | Content searching |
| `patch` | Complete | Unified diff application |
| `memory_get` | Complete | Memory retrieval |
| `memory_search` | Complete | Memory search |
| 3 plugin tools | Complete | Via rockbot-tools-mcp, rockbot-tools-credentials, rockbot-tools-markdown |

#### Missing Tools
- [ ] `web_search` - Web searching
- [ ] `web_fetch` - URL fetching
- [ ] `browser_navigate` - Browser automation
- [ ] `browser_screenshot` - Page capture

#### Missing Infrastructure
- [ ] Sandboxed execution
- [ ] Before/after tool hooks
- [ ] Tool timeout handling
- [ ] **Credential injection into tool execution context**
- [ ] Tool result sanitization

---

### 6. Plugin System (Section 9)

#### Implemented
- [x] `PluginManager` struct
- [x] `PluginManifest` schema
- [x] Load/unload lifecycle methods
- [x] Tool/channel definition extraction

#### Missing
- [ ] **Actual plugin execution** (no WASM, no native)
- [ ] Hook registration and dispatch
- [ ] HTTP route registration
- [ ] Gateway method extension
- [ ] CLI command extension
- [ ] Service lifecycle management
- [ ] Plugin isolation/sandboxing
- [ ] Plugin discovery (global, workspace, bundled)
- [ ] Plugin configuration injection

---

### 7. Cron System (Section 13)

#### Implemented
- [x] `heartbeat_interval` config field
- [x] `last_heartbeat` tracking

#### Missing (entire subsystem)
- [ ] Cron job schema
- [ ] Schedule types (at, every, cron expression)
- [ ] Job persistence
- [ ] Scheduler loop
- [ ] Job execution
- [ ] Payload types (systemEvent, agentTurn)
- [ ] Delivery modes (none, announce, webhook)
- [ ] Job state tracking (nextRun, lastRun, errors)
- [ ] CLI: `cron list/add/edit/remove/run`
- [ ] Gateway RPC: `cron.*` methods

---

### 8. Skills System (Section 16)

#### Implemented
- Nothing

#### Missing (entire subsystem)
- [ ] Skill definition schema
- [ ] Skill discovery (bundled, workspace, agent-specific)
- [ ] Skill prompt injection
- [ ] Install specifications (brew, node, go, uv, download)
- [ ] Skill invocation policy
- [ ] Skill metadata (always, requires, os filters)

---

### 9. Media Pipeline (Section 17)

#### Implemented
- Nothing

#### Missing (entire subsystem)
- [ ] Media type detection
- [ ] Image processing (resize, convert to JPEG)
- [ ] Audio transcription (STT)
- [ ] TTS synthesis
- [ ] Video thumbnail extraction
- [ ] Document text extraction (PDF, etc.)
- [ ] Media caching
- [ ] Per-channel media format adaptation

---

### 10. Configuration System (Section 10)

#### Implemented
- [x] JSON5 config parsing
- [x] Gateway config section
- [x] Credentials config section
- [x] Agent definitions with temperature/max_tokens
- [x] Logging config (partial)
- [x] Hot-reload file watcher (watcher exists; apply not wired)

#### Missing
- [ ] **Config hot apply** — watcher fires but changes are not applied to running gateway
- [ ] Full `auth.profiles` handling
- [ ] Model definitions and aliases
- [ ] Session config (scope, idleMinutes, typingMode, reset)
- [ ] Per-channel settings parsing
- [ ] Tool allow/disallow lists
- [ ] Environment variable expansion
- [ ] Config validation against schema
- [ ] CLI: `config get/set/apply/schema`

---

### 11. CLI Commands (Section 11)

#### Implemented
| Command | Status |
|---------|--------|
| `gateway run` | Working |
| `gateway dev` | Partial |
| `credentials *` | Full suite |
| `doctor` | Basic checks |
| `session *` | Basic ops |
| `agent *` | Working |
| `tool *` | Working |
| `config *` | Partial |

#### Missing Commands
- [ ] `setup` - Workspace initialization
- [ ] `onboard` - Interactive wizard
- [ ] `configure` - Guided config
- [ ] `status` - Channel health overview
- [ ] `message send` - Direct message sending
- [ ] `channels` - Channel management
- [ ] `models` - Model configuration
- [ ] `cron` - Cron job management
- [ ] `plugins` - Plugin management

---

### 12. Security Model (Section 14)

This is the **most complete** section alongside the agent system.

#### Implemented
- [x] Master key derivation (Argon2id)
- [x] Credential encryption (AES-256-GCM)
- [x] Credential storage with nonces
- [x] 4-tier permission levels
- [x] Path pattern matching (glob)
- [x] Permission evaluation
- [x] HIL approval queue
- [x] HIL notification channel
- [x] Hash-chained audit log
- [x] Audit log verification
- [x] Multiple unlock methods (password, keyfile, Age)
- [x] TUI and Web UI for credential management

#### Missing
- [ ] Keyring integration (macOS Keychain, etc.)
- [ ] YubiKey/hardware key support
- [ ] 2FA for AllowHIL2FA level
- [ ] Response sanitization (credential stripping)
- [ ] Memory protection (mlock)
- [ ] Credential rotation
- [ ] Mobile push notifications for HIL
- [ ] **Credential injection into tool execution context**

---

### 13. User Interface (Section 20) — TUI + Web UI

The UI system is **~75% complete** overall, with the TUI slightly more mature than the Web UI.

#### TUI Implementation Status

| Component | Status | Notes |
|-----------|--------|-------|
| **App Loop** | Complete | Crossterm + async message channel |
| **State Management** | Complete | Elm-like AppState + Message pattern |
| **Sidebar Navigation** | Complete | 6 sections, keyboard nav |
| **Dashboard** | Complete | Gateway status, agent list, vault status |
| **Credentials** | Complete | 4 sub-tabs, full CRUD, unlock flows |
| **Agents** | ~85% | List view, CRUD modals, config editing |
| **Sessions** | ~80% | List works, per-session chat with tool call display |
| **Models** | ~70% | Provider cards, config editing, live Bedrock polling |
| **Settings** | ~60% | Display with real gateway data binding |
| **Modals** | Complete | Password, confirm, add/edit agent forms |

#### TUI Missing Features
- [ ] Chat streaming display (chunks arrive, not rendered incrementally)
- [ ] Real-time WebSocket updates
- [ ] Channels status view
- [ ] Cron jobs view
- [ ] Plugins view
- [ ] Model testing/validation

#### Web UI Implementation Status

| Component | Status | Notes |
|-----------|--------|-------|
| **Gateway Server** | Complete | Embedded HTML SPA in `web_ui.rs` (~1645 lines) |
| **Layout** | Complete | Sidebar + main content, 6 sections |
| **Color Palette** | Complete | Cyberpunk dark theme |
| **Dashboard** | Complete | Stats cards, agent table |
| **Credentials** | Complete | Init, unlock, endpoints CRUD |
| **Sessions** | ~60% | List + chat UI, per-session state |
| **Agents** | ~65% | List view, schema-driven provider config |
| **Models** | ~70% | Provider cards with config |
| **Settings** | ~50% | Display with real data |
| **Keyboard Shortcuts** | Complete | 1-6 for navigation |

#### Web UI Missing Features
- [ ] WebSocket integration (currently REST polling)
- [ ] Real-time chat streaming
- [ ] Form validation
- [ ] Error toast notifications
- [ ] Mobile responsive breakpoints
- [ ] Touch-friendly interactions
- [ ] Credential permission editing
- [ ] Agent binding configuration

#### Shared Infrastructure Gaps
- [ ] **State Sync**: No WebSocket subscription for real-time updates
- [ ] **Streaming**: LLM streaming not wired through to UI layer
- [ ] **Theme System**: No light mode or user customization
- [ ] **i18n**: No internationalization support

---

## UI Extension Playbook

When adding new features, follow these patterns to maintain consistency:

### Adding a New Navigation Section

**Example: Adding "Channels" section**

1. **State** (`state.rs`):
   ```rust
   pub enum MenuItem {
       // ... existing
       Channels,  // Add variant
   }

   impl MenuItem {
       pub fn all() -> Vec<Self> {
           vec![..., Self::Channels]  // Add to list
       }
       pub fn icon(&self) -> &'static str {
           Self::Channels => "~",
       }
   }
   ```

2. **TUI Component** (`components/channels.rs`):
   ```rust
   pub fn render_channels(frame: &mut Frame, area: Rect, state: &AppState) {
       // Render channel list, status indicators
   }
   ```

3. **Web UI** (`web_ui.rs`):
   ```html
   <li class="nav-item" data-page="channels">
       <span class="icon">~</span> Channels
   </li>

   <div id="page-channels" class="content page hidden">
       <!-- Channel list, status cards -->
   </div>
   ```
   ```javascript
   function loadChannelsPage() {
       api('/api/channels/status').then(renderChannels);
   }
   ```

4. **API Endpoint** (`gateway.rs`):
   ```rust
   "/api/channels/status" => {
       let status = get_channel_status().await;
       json_response(&status)
   }
   ```

### Adding Sub-tabs

**Example: Adding "Bindings" sub-tab to Agents**

1. **State**:
   ```rust
   pub enum AgentsTab { List, Config, Bindings }
   ```

2. **TUI**: Handle `[`/`]` in `handle_normal_mode()`:
   ```rust
   MenuItem::Agents => {
       self.agents_tab = (self.agents_tab + 1) % 3;
   }
   ```

3. **Web UI**: Add tab bar and content switching

### Adding Real-time Updates

When a feature needs live updates:

1. **Gateway**: Broadcast events via WebSocket
2. **TUI**: Handle in async message loop
3. **Web UI**: Subscribe to WebSocket, update DOM

```javascript
// Web UI WebSocket pattern
const ws = new WebSocket(`ws://${location.host}/ws`);
ws.onmessage = (e) => {
    const event = JSON.parse(e.data);
    if (event.type === 'channel_status') updateChannelUI(event.payload);
};
```

### Adding a Modal Form

1. **State**: Add `InputMode` variant
2. **TUI**: Create `render_*_modal()` + `handle_*()`
3. **Web UI**: Add modal HTML + show/close functions
4. **Validation**: Implement in both UIs consistently

### File Locations Reference

| Feature | TUI | Web UI |
|---------|-----|--------|
| State types | `tui/state.rs` | JS globals in `web_ui.rs` |
| App loop | `tui/app.rs` | `<script>` in `web_ui.rs` |
| Components | `tui/components/*.rs` | Inline in HTML |
| Modals | `tui/components/modals.rs` | Modal divs in HTML |
| API endpoints | `core/gateway.rs` | Same file |
| Styles | N/A (ratatui) | `<style>` in HTML |

---

## What Needs Implementation

### High Priority
1. **Streaming through agent/gateway/UI** — LLM layer has streaming in all 3 providers; it is not wired through the agent execution loop, gateway API, or UI display
2. **Credential injection into tool execution context** — credentials stored and managed, not yet passed into tool calls
3. **Channel routing/binding system** — channels work in isolation, no binding resolution or route matching
4. **Subagent delegation mechanism** — agent config supports parent_id but delegation is not executed
5. **WebSocket protocol for real-time updates** — upgrade path exists, functional WebSocket not implemented

### Medium Priority
1. **Web tools** (`web_search`, `web_fetch`) — 9 built-in tools work, web tools missing
2. **Model failover chains** — single model per agent, no fallback
3. **Cron scheduler** — config field exists, no execution engine
4. **Skills system** — entirely absent
5. **Config hot apply** — file watcher fires, changes not applied to running gateway

---

## Priority Recommendations

### Phase 1: Core Agent Loop (COMPLETED)
- Streaming responses — implemented at LLM layer for all 3 providers
- Tool execution loop — multi-tool per turn with iteration tracking
- System prompt assembly — SOUL.md and SYSTEM-PROMPT.md injection
- Error handling — structured retry with exponential backoff, error classification, and jitter

### Phase 2: End-to-End Streaming + Routing (High Priority)
1. Wire streaming from LLM layer through agent loop to gateway response and UI
2. Channel binding system and route resolution
3. Credential injection into tool context
4. Subagent delegation

### Phase 3: Automation (Medium Priority)
1. Cron scheduler
2. Web tools (web_search, web_fetch)
3. Config hot apply
4. Skills system

### Phase 4: Polish (Lower Priority)
1. Plugin execution (WASM or native)
2. Media pipeline
3. Remaining CLI commands (models, cron, plugins, message)
4. WebSocket real-time updates in Web UI

---

## LLM Providers

All three providers are working with streaming support at the LLM layer:

| Provider | Status | Streaming | Notes |
|----------|--------|-----------|-------|
| Anthropic | Working | Implemented | SSE streaming at LLM layer |
| OpenAI | Working | Implemented | SSE streaming at LLM layer |
| Bedrock | Working | Implemented | Live model polling; `aws_smithy_types::Document` manual converters |

Streaming is **not yet wired** through the agent execution loop or gateway API to the UI.

---

## Lines of Code by Crate

| Crate | Primary Purpose |
|-------|-----------------|
| `rockbot-core` | Gateway, session, agent, config |
| `rockbot-credentials` | Vault, permissions, audit |
| `rockbot-cli` | Commands, TUI |
| `rockbot-llm` | Anthropic, OpenAI, Bedrock providers |
| `rockbot-channels` | Channel trait, ChannelRegistry |
| `rockbot-channels-discord` | Discord via Serenity |
| `rockbot-channels-telegram` | Telegram via Teloxide |
| `rockbot-channels-signal` | Signal placeholder |
| `rockbot-tools` | Tool trait, ToolProviderRegistry |
| `rockbot-tools-credentials` | Credential tool plugin |
| `rockbot-tools-mcp` | MCP tool plugin |
| `rockbot-tools-markdown` | Markdown tool plugin |
| `rockbot-memory` | Memory system |
| `rockbot-plugins` | Plugin manager scaffold |
| `rockbot-security` | Capabilities, context |
| `rockbot-credentials-schema` | Shared CredentialSchema types (leaf crate) |
| `rockbot` | Binary, feature flag passthrough |
| `rockbot-credentials-schema` | Leaf crate, only serde dep |

Total: 18 crates, ~35,000 LOC

---

## Technical Debt

### Known TODOs
- Gateway uptime/memory tracking returns 0 (`uptime_seconds: 0` — TODO: track actual uptime)
- `openclaw` command reference in gateway control (should be `rockbot`)
- SSH agent unlock path is a stub (`// TODO: Actually unlock via SSH agent`)
- Keyring support stub in gateway (`// TODO: Implement keyring support`)

### Missing Tests
- [ ] Integration tests for gateway protocol
- [ ] E2E tests for full message flow
- [ ] Channel adapter tests
- [ ] Credential injection tests
- [ ] Cron scheduler tests
- [ ] Streaming pipeline tests

### Documentation Gaps
- [ ] API documentation (rustdoc incomplete)
- [ ] User guide
- [ ] Channel setup guides
- [ ] Plugin development guide

---

## Conclusion

The foundation is solid and significantly more complete than the initial implementation: the gateway runs with 30+ HTTP endpoints, sessions persist with per-session chat state, credentials are fully encrypted, Discord and Telegram channels work, all three LLM providers (Anthropic, OpenAI, Bedrock) are operational with streaming at the LLM layer, 9 built-in tools are complete, and both TUI and Web UI have full 6-section layouts backed by real gateway data.

The path forward is:

1. **Wire streaming end-to-end** (LLM layer through agent loop through gateway through UI)
2. **Build the channel binding/routing system** (channels work but routing does not)
3. **Inject credentials into tool execution context**
4. **Add cron scheduler** (enable automation use cases)
5. **Implement subagent delegation**

Estimated effort to reach SPEC.md parity: **2-3 months** at current pace, or **4-6 weeks** with focused full-time development.
