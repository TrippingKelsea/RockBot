#!/usr/bin/env bash
# Info paths must not open network sockets.
source "$(dirname "$0")/lib.sh"
require_binary
require_strace "no network on help paths" || { summary; exit 0; }

assert_strace_absent "--help: no network calls" "network" "connect|socket\(AF_INET|bind" --help
assert_strace_absent "--version: no network calls" "network" "connect|socket\(AF_INET|bind" --version

summary
