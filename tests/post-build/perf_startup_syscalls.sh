#!/usr/bin/env bash
# Total syscall budget for --help. Calibrate threshold after first run.
# Baseline: TBD (run and update after first measurement)
source "$(dirname "$0")/lib.sh"
require_binary
require_strace "startup syscall budget" || { summary; exit 0; }

# Budget: 500 syscalls for --help (calibrate after first run)
assert_strace_max "--help total syscall budget" 500 --help

summary
