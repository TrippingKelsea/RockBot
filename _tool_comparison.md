# Tool Comparison: RockBot vs Strands Agents SDK

**Updated:** 2026-03-14

---

## Direct Equivalents (parity)

| Strands Tool | RockBot Tool | Notes |
|---|---|---|
| `file_read` | `read` | Both support offset/limit |
| `file_write` | `write` | Equivalent |
| `editor` | `edit` | Both do search/replace; RockBot also has `patch` for unified diffs |
| `shell` | `exec` | Shell command execution |
| `http_request` | `web_fetch` | RockBot strips HTML to text; Strands returns raw |
| `retrieve` (RAG) | `memory_get` / `memory_search` | RockBot uses TF-IDF + embedding hybrid; Strands plugs into Bedrock KB |
| `use_llm` / sub-agent | `invoke_agent` | Both support delegation with depth limits |
| `swarm` | `handoff` + `blackboard_read/write` | RockBot decomposes into composable primitives; Strands wraps as one tool |
| `agent_graph` / workflow | Workflow engine (config-driven DAG) | RockBot is config-driven with parallel fan-out; Strands exposes as a tool |
| `web_search` | `web_search` | Strands defaults to Amazon; RockBot uses Brave Search API |

---

## Strands has, RockBot doesn't

| Strands Tool | What it does | Priority | Recommendation |
|---|---|---|---|
| `python_repl` | Execute Python code in-process | **High** | Add as `repl` tool with language param. Many agent tasks benefit from computation, data wrangling, plotting. Wire through `sandbox.rs` container infra for safe execution |
| `calculator` | Math expression evaluation | **Medium** | Tiny standalone tool. Good fallback when sandbox/REPL isn't available |
| `current_time` | Return current date/time/timezone | **High** | Trivial (~20 lines). Surprisingly useful since LLMs don't know the current time |
| `code_interpreter` | Sandboxed code execution with output capture | **High** | Sandboxed version of python_repl. Our `sandbox.rs` container infra exists but isn't wired as a tool yet |
| `environment` | Read/set environment variables | **Low** | Security risk. `exec` can do `echo $VAR` when needed. Skip |
| `knowledge_base` | Bedrock Knowledge Base retrieval | **Low** | AWS-specific. Only useful if targeting Bedrock users. Could add as optional provider behind feature flag |
| `nova_canvas` | Image generation via Amazon Nova | **Skip** | Vendor-specific, niche |
| `nova_rerank` | Rerank search results | **Skip** | Belongs in retrieval pipeline, not as a user-facing tool |
| `load_tool` | Dynamically load tools at runtime | **Low** | We have MCP for dynamic tools. A `load_tool_from_code` would be interesting but risky |
| `cron` | Schedule future actions as a tool | **Skip (as tool)** | We have cron as gateway infrastructure. Letting the LLM schedule arbitrary cron jobs is a security concern |
| `guardrails` | Content moderation tool | **Skip (as tool)** | We run guardrails as a pipeline, not something the LLM calls on itself |

---

## RockBot has, Strands doesn't

| RockBot Tool | What it does | Why it matters |
|---|---|---|
| `glob` | File pattern matching | Strands relies on `shell` + `find`; dedicated tool is faster and safer |
| `grep` | Regex content search | Strands relies on `shell` + `grep`; dedicated tool avoids shell injection |
| `patch` | Apply unified diffs | Enables large multi-hunk edits in one call |
| `browser` | Headless Chrome automation | Full browser interaction, not just HTTP fetching |
| `test` | Auto-detect and run test suites | Language-aware test runner |
| `lint` | Auto-detect and run linters | Language-aware linter |
| `clarify` | Ask the user a question (HIL) | Structured way for agent to request clarification |
| `blackboard_read/write` | Shared swarm state | Explicit primitives vs Strands' opaque swarm wrapper |
| `handoff` | Transfer conversation control | Composable orchestration primitive |

---

## Recommended additions (prioritized)

### Tier 1 - Add now (high value, low effort)

1. **`current_time`** - ~20 lines. Returns ISO 8601 datetime, timezone, Unix timestamp. Every agent needs this.
2. **`calculator`** - Math expression evaluator. Eliminates common LLM arithmetic errors. No external deps needed.

### Tier 2 - Add soon (high value, moderate effort)

3. **`code_interpreter`** / `repl` - Sandboxed code execution via `sandbox.rs` container infra. Returns stdout, stderr, generated files. Python first, then extensible to other languages.

### Tier 3 - Consider later (situational value)

4. **`knowledge_base`** - Bedrock KB integration behind `bedrock` feature flag. Only if we want deeper AWS ecosystem support.
5. **`load_tool`** - Dynamic tool loading from code. Powerful but needs careful sandboxing.

---

## Profile membership

Current profiles and their tools:

| Profile | Tools |
|---|---|
| **minimal** | read, write |
| **standard** | read, write, edit, exec, glob, grep, patch, invoke_agent, handoff, web_fetch, web_search, test, lint, clarify |
| **full** | All standard + memory_get, memory_search, browser, blackboard_read, blackboard_write |

Proposed additions:
- `current_time` -> **all profiles** (minimal, standard, full)
- `calculator` -> **standard** and **full**
- `code_interpreter` -> **full** only (requires sandbox)

---

## Summary

RockBot has **19 built-in tools** + MCP proxy tools + 3 plugin crate tools.
Strands has **~20 built-in tools** + Bedrock-specific extensions.

RockBot is ahead on **developer/coding tools** (glob, grep, patch, test, lint, browser) and **orchestration primitives** (handoff, blackboard, invoke_agent as separate composable tools).

Strands is ahead on **computational tools** (python_repl, code_interpreter, calculator) and **convenience tools** (current_time).

Adding `current_time`, `calculator`, and `code_interpreter` closes all meaningful gaps. Everything else Strands has is either vendor-specific (Nova), already covered by existing tools, or infrastructure we have in a different form (guardrails pipeline, cron system, MCP for dynamic tools).
