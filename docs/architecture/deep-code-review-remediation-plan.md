# Deep Code Review Remediation Plan

## Purpose

This document validates the findings in `CODE_REVIEW.md` against current
RockBot `HEAD` and turns the still-live issues into an implementation plan.

It is intentionally not a copy of the review. Some review items are still live,
some are only partially true, and some are stale against the current codebase.

## Validation Summary

This document was re-checked after the `rockbot.data` virtual-disk foundation
landed in `d54ee5c`. The storage refactor did not materially change the review
findings below; the remaining work is still concentrated in security
boundaries, provider correctness, and long-running resource behavior.

### Still-Live Critical Findings

The following review findings were validated directly against the current code:

1. Filesystem capability `"."` currently grants effectively unrestricted access
   in [rockbot-security/src/lib.rs](../../crates/rockbot-security/src/lib.rs).
2. Sandbox path preflight checks still read `params["path"]`, while file tools
   primarily use `file_path`, in
   [rockbot-tools/src/lib.rs](../../crates/rockbot-tools/src/lib.rs).
3. SSH vault wrapping is derived from public-key material only in
   [rockbot-credentials/src/storage.rs](../../crates/rockbot-credentials/src/storage.rs).
4. Agent content truncation still slices UTF-8 by bytes in
   [rockbot-agent/src/agent.rs](../../crates/rockbot-agent/src/agent.rs).
5. The credential tool still retrieves a secret and discards it in
   [rockbot-tools-credentials/src/lib.rs](../../crates/rockbot-tools-credentials/src/lib.rs).
6. `McpServerManager::Drop` is still effectively a no-op in
   [rockbot-tools-mcp/src/lib.rs](../../crates/rockbot-tools-mcp/src/lib.rs).
7. Gateway file-level Clippy suppressions are still present in
   [rockbot-gateway/src/gateway.rs](../../crates/rockbot-gateway/src/gateway.rs).
8. WebSocket outbound connections still use unbounded channels in
   [rockbot-gateway/src/gateway.rs](../../crates/rockbot-gateway/src/gateway.rs).
9. TUI vault unlock still writes debug information to
   `/tmp/rockbot_debug.log` in
   [rockbot-tui/src/app.rs](../../crates/rockbot-tui/src/app.rs).
10. Age identity input is still unmasked in the TUI in
    [rockbot-tui/src/app.rs](../../crates/rockbot-tui/src/app.rs).
11. Bedrock JSON number conversion still maps negative integers to unsigned
    values in [rockbot-llm-bedrock/src/lib.rs](../../crates/rockbot-llm-bedrock/src/lib.rs).
12. Anthropic conversation conversion still flattens multi-turn role structure
    into a single string in
    [rockbot-llm-anthropic/src/lib.rs](../../crates/rockbot-llm-anthropic/src/lib.rs).
13. Delegation depth is still reset to `0` in tool execution context in
    [rockbot-agent/src/agent.rs](../../crates/rockbot-agent/src/agent.rs).
14. Security config fields `allowed_commands` and `blocked_commands` still do
    not appear to be enforced in
    [rockbot-security/src/lib.rs](../../crates/rockbot-security/src/lib.rs).
15. CLI credential add still hardcodes password unlock in
    [rockbot-cli/src/commands/credentials.rs](../../crates/rockbot-cli/src/commands/credentials.rs).
16. PKI key creation still has a create-then-chmod TOCTOU window in
    [rockbot-pki/src/backend.rs](../../crates/rockbot-pki/src/backend.rs) and
    [rockbot-pki/src/manager.rs](../../crates/rockbot-pki/src/manager.rs).
17. `std::env::set_var` is still called from async vault-unlock flow in
    [rockbot-cli/src/commands/vault_unlock.rs](../../crates/rockbot-cli/src/commands/vault_unlock.rs).
18. Gateway still contains a production `assert!` on `agent_id` in
    [rockbot-gateway/src/gateway.rs](../../crates/rockbot-gateway/src/gateway.rs).
19. Config env-var expansion still loops recursively in
    [rockbot-config/src/config.rs](../../crates/rockbot-config/src/config.rs).
20. The OpenAI provider test failure is still live:
    `crate::ImageContent` import is wrong in
    [rockbot-llm-openai/src/lib.rs](../../crates/rockbot-llm-openai/src/lib.rs).

### Partially True or Reframed Findings

These findings are directionally valid, but need narrower framing:

1. Enrollment bootstrap still performs an initial insecure CA fetch in
   [rockbot-cli/src/commands/cert.rs](../../crates/rockbot-cli/src/commands/cert.rs),
   but first-time enrollment now requires an explicit CA fingerprint when no
   local CA file exists. The remaining issue is TOFU-style transport during the
   CA fetch, not silent unconditional trust.
2. The gateway file-level Clippy allow in
   [rockbot-gateway/src/gateway.rs](../../crates/rockbot-gateway/src/gateway.rs)
   is policy debt and safety debt, but not a direct vulnerability by itself.
3. Client TLS does not silently downgrade to insecure transport anymore, but
   absence of a valid CA still causes connector construction to fail closed via
   `None` in [rockbot-client/src/client.rs](../../crates/rockbot-client/src/client.rs).

### Stale Findings

These review items are stale against current code:

1. `rockbot-agent` and `rockbot-gateway` Clippy debt from the previous review
   batch has already been burned down to pass `cargo clippy -D warnings` for
   those crates.
2. The `rockbot-tools` unit-test build failure from `DetectedCommand` has
   already been fixed.
3. The `todo!()` cited in `rockbot-agent/src/indexer.rs` is inside a test
   fixture snippet, not executable production code.

## High-Priority Important Findings Also Validated

The following non-critical findings were spot-checked and are still live:

1. The client listener still exposes a large authenticated API surface in
   [rockbot-gateway/src/gateway.rs](../../crates/rockbot-gateway/src/gateway.rs).
2. Streaming output guardrails still run after chunks have already been emitted
   in [rockbot-agent/src/agent.rs](../../crates/rockbot-agent/src/agent.rs).
3. Compaction LLM calls still lack an explicit timeout in
   [rockbot-agent/src/agent.rs](../../crates/rockbot-agent/src/agent.rs).
4. Nudge/final-warning messages are still persisted as `User` messages in
   [rockbot-agent/src/agent.rs](../../crates/rockbot-agent/src/agent.rs).
5. Docker/direct sandbox timeout paths still return without explicitly killing
   child processes in [rockbot-agent/src/sandbox.rs](../../crates/rockbot-agent/src/sandbox.rs).
6. Enrollment token comparison is now constant-time for equal-length values,
   but length is still checked first in
   [rockbot-pki/src/index.rs](../../crates/rockbot-pki/src/index.rs).
7. CRL number and revocation timestamps are still incorrect in
   [rockbot-pki/src/ca.rs](../../crates/rockbot-pki/src/ca.rs).
8. Argon2 still uses library defaults in
   [rockbot-credentials/src/crypto.rs](../../crates/rockbot-credentials/src/crypto.rs).

## Remediation Phases

## Phase 0: Immediate Build and Crash-Correctness Fixes

These are low-risk and should land first.

1. Fix the OpenAI provider test import in
   [rockbot-llm-openai/src/lib.rs](../../crates/rockbot-llm-openai/src/lib.rs).
2. Replace agent UTF-8 byte slicing with a shared safe truncation helper.
3. Replace gateway `assert!` input enforcement with normal validation errors.
4. Mask Age identities in the TUI.
5. Remove `/tmp/rockbot_debug.log` writes or gate them behind an explicit debug
   feature/env flag.

## Phase 1: Security Boundary Fixes

These are the highest-risk live issues.

1. Fix capability `"."` semantics in
   [rockbot-security/src/lib.rs](../../crates/rockbot-security/src/lib.rs).
   Recommended model:
   - resolve `"."` to the session workspace root
   - canonicalize request paths before comparison
   - never treat `"."` as unconditional allow
2. Fix tool sandbox preflight parameter matching in
   [rockbot-tools/src/lib.rs](../../crates/rockbot-tools/src/lib.rs).
   Recommended model:
   - centralize a per-tool path extractor
   - cover `file_path`, `path`, `workdir`, and patch/edit aliases
3. Replace SSH vault wrapping in
   [rockbot-credentials/src/storage.rs](../../crates/rockbot-credentials/src/storage.rs).
   Recommended direction:
   - use Age recipients or a real SSH-based recipient encryption mechanism
   - do not derive vault wrapping keys from public-key bytes
4. Make the credential tool actually return usable secret material in a safe
   interface, or remove the tool if the security model does not permit that.
5. Wire `allowed_commands` / `blocked_commands` into actual enforcement.

## Phase 2: Process and Resource Hardening

1. Replace gateway unbounded outbound WS channels with bounded channels and
   explicit backpressure/drop policy.
2. Ensure sandbox timeout paths kill child/container processes.
3. Implement real MCP child cleanup on `Drop`, including best-effort kill/wait.
4. Propagate real delegation depth through tool execution context.
5. Add explicit timeout to semantic compaction LLM calls.

## Phase 3: Provider Correctness

1. Fix Bedrock negative integer handling:
   - map signed integers to signed document numbers if supported
   - otherwise preserve as string or fail explicitly
2. Rework Anthropic request conversion to preserve real role/message structure
   instead of flattening the transcript into one string.
3. Revisit streaming guardrails so blocked output is not emitted before the
   guardrail decision.

## Phase 4: PKI and Enrollment Hardening

1. Eliminate create-then-chmod TOCTOU windows for PKI key files and storage
   keys.
2. Tighten first-contact enrollment bootstrap so CA retrieval does not rely on
   insecure transport without an out-of-band verification step.
3. Remove the enrollment-token length side-channel by comparing fixed-size
   digests or normalizing token length before constant-time comparison.
4. Fix CRL numbering and revocation timestamps.

## Phase 5: Config and Runtime Safety

1. Replace recursive env-var expansion with a bounded non-recursive expansion
   model in [rockbot-config/src/config.rs](../../crates/rockbot-config/src/config.rs).
2. Remove `std::env::set_var` from async runtime paths and return credentials
   through explicit runtime config/state instead.
3. Update CLI credential-add to unlock according to the vault’s configured
   unlock method instead of assuming password unlock.
4. Revisit the large client-listener API surface and decide which operations
   should require stronger identity/role checks.

## Suggested Execution Order

1. OpenAI test import
2. UTF-8 truncation
3. gateway `assert!` removal
4. TUI secret exposure fixes
5. capability `"."` semantics and path canonicalization
6. sandbox parameter-key fix and command/path extraction hardening
7. SSH vault redesign
8. credential tool fix
9. allowed/blocked commands enforcement
10. WS backpressure
11. sandbox kill-on-timeout
12. MCP process cleanup
13. delegation depth propagation
14. Bedrock integer handling
15. Anthropic multi-turn preservation
16. compaction timeout + streaming guardrail hardening
17. PKI file creation hardening
18. enrollment hardening follow-up
19. CRL fixes
20. config expansion + `set_var` removal
21. CLI vault unlock correction

## Current Execution Batches

The implementation work should be landed in these checkpoint batches:

1. Crash/correctness and secret-exposure fixes
   - OpenAI test import
   - UTF-8-safe truncation
   - gateway `assert!` replacement
   - remove `/tmp/rockbot_debug.log`
   - mask Age identities
2. Filesystem and process-boundary fixes
   - capability `"."` semantics
   - canonical path enforcement
   - tool sandbox parameter extraction
   - `allowed_commands` / `blocked_commands` enforcement
3. Vault and credential fixes
   - stop public-key-derived SSH wrapping
   - fix credential tool return payload
   - fix CLI unlock-method assumptions
4. Resource and runtime hardening
   - WS backpressure
   - MCP cleanup
   - sandbox kill-on-timeout
   - remove async `set_var`
5. Provider correctness fixes
   - Bedrock integer conversion
   - Anthropic message structure
   - compaction timeout / streaming guardrails
6. PKI and config hardening
   - key-file creation race
   - config env expansion
   - remaining enrollment / CRL fixes

## Verification Checklist

Each remediation batch should include targeted checks:

- `cargo test -p rockbot-llm-openai --lib`
- `cargo test -p rockbot-tools --lib`
- `cargo clippy -p rockbot-security -p rockbot-tools -p rockbot-agent -p rockbot-gateway --all-targets --no-deps -- -D warnings`
- targeted integration tests for:
  - capability/path enforcement
  - SSH/age vault unlock
  - websocket backpressure
  - sandbox timeout cleanup
  - provider conversation conversion

## Summary

The revised deep review found several real remaining issues, especially around
filesystem trust boundaries, SSH vault design, provider correctness, and
resource/backpressure behavior. It also overstated some already-fixed items.

The current highest-priority work is:

1. repair the real security boundaries
2. fix provider correctness and secret-handling bugs
3. harden long-running resource behavior
4. then clean up remaining PKI/config/runtime debt
