# Runlog

## 2026-03-18

- Documentation/CLI mismatch: certificate generation examples are using the wrong invocation form.
  - `rockbot` was used as if it were on `PATH`, but the actual local-build workflow in this repo is `./rockbot`.
  - `cert client generate` was documented/invoked with `--name gateway`, but the CLI actually expects a positional `<NAME>`.
  - Verified working form:
    - `./rockbot cert client generate gateway --role gateway --san 127.0.0.1 --san 172.30.200.146 --san localhost --days 784`

- CLI capability gap: `cert enroll create` does not currently allow multiple roles.
  - Attempted:
    - `./rockbot cert enroll create --role client --role tui --role agent --uses 1 --expires 24h`
  - Actual result:
    - `error: the argument '--role <ROLE>' cannot be used multiple times`
  - Desired future behavior:
    - allow multiple roles on enrollment token creation
    - update docs/help text accordingly once implemented

- Storage/runtime mismatch: bad embedded stores were still being "repaired" by directly opening them.
  - Observed:
    - `gateway run` and `storage repair` could still abort in `redb` even after storage planning was added
  - Root cause:
    - repair/startup paths were still touching suspect vdisk volumes in-process
  - Desired behavior:
    - probe out-of-process
    - reimport from legacy when available
    - otherwise quarantine/remove the suspect volume and fall back cleanly

- PKI layout mismatch: generated gateway TLS materials and vault keyfile were escaping the `pki/` hierarchy.
  - Observed paths:
    - `~/.config/rockbot/gateway.crt`
    - `~/.config/rockbot/gateway.key`
    - `~/.config/rockbot/vault.key`
  - Desired behavior:
    - gateway TLS under `pki/certs` and `pki/keys`
    - vault key under `pki/keys`

- Agent memory storage mismatch: agent startup was eagerly creating persistence under the configured workspace tree.
  - Observed path:
    - `~/.config/rockbot/workspace/memory/`
  - Desired behavior:
    - no eager workspace-tree materialization on startup
    - managed storage path first
    - eventually move durable agent memory under `rockbot.data`
