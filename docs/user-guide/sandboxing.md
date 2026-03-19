# Sandboxing Tools

RockBot's hardened code-execution path is designed around two external runtimes:

- Firecracker microVMs for strong process isolation
- Wasmtime for executing WebAssembly workloads inside the guest boundary

This page documents the host prerequisites an operator should install before
enabling the secure code-execution path.

## Requirements

- Linux host with KVM enabled
- `firecracker` binary installed and runnable by the RockBot service user
- `jailer` binary installed if you want Firecracker's recommended jailed launch flow
- `wasmtime` installed for guest-side wasm execution and local validation
- A Firecracker-compatible kernel image and rootfs image

## Install Firecracker

Follow the official Firecracker getting-started documentation:

- Firecracker repository: <https://github.com/firecracker-microvm/firecracker>
- Getting started: <https://github.com/firecracker-microvm/firecracker/blob/main/docs/getting-started.md>

At minimum, verify these commands succeed on the host:

```bash
firecracker --version
jailer --version
```

You also need:

- `/dev/kvm` accessible to the service account
- a minimal kernel image
- a root filesystem image prepared for the microVM

## Install Wasmtime

Follow the official Wasmtime documentation:

- Wasmtime crate docs: <https://docs.rs/wasmtime/latest/wasmtime/>
- Project site: <https://wasmtime.dev/>

Verify the runtime is available:

```bash
wasmtime --version
```

If you plan to execute WASI programs, install a build/runtime combination that
supports WASI and keep the CLI version aligned with the embedded runtime version
used by the RockBot deployment.

## Host Preparation Checklist

Before enabling secure code execution, confirm:

```bash
test -r /dev/kvm
firecracker --version
wasmtime --version
```

And ensure the following artifacts exist on disk:

- Firecracker kernel image
- Firecracker rootfs image
- a writable workspace directory to mount into the guest
- a directory for per-run VM state such as sockets, FIFOs, and logs

## Operational Notes

- Firecracker is the isolation boundary. Wasmtime should run inside the guest
  for untrusted code execution rather than directly on the host.
- Keep Firecracker and Wasmtime updated together during maintenance windows.
- Treat the kernel image and rootfs as part of your trusted computing base.
- Validate KVM access and microVM startup in staging before enabling it for
  interactive agents.
