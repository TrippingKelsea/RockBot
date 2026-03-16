#!/usr/bin/env bash
# Regression guard: help/version paths must not construct a Tokio runtime.
# Tokio runtime creation triggers clone3 (thread spawning), epoll_create1,
# and eventfd2 syscalls. These must be absent on info-only paths.
source "$(dirname "$0")/lib.sh"
require_binary
require_strace "help paths skip Tokio runtime" || { summary; exit 0; }

assert_strace_zero "--help: no clone3 (thread spawning)" "clone3" --help
assert_strace_zero "--help: no epoll_create1 (async reactor)" "epoll_create1" --help
assert_strace_zero "--help: no eventfd2 (async wakeup)" "eventfd2" --help

assert_strace_zero "--version: no clone3" "clone3" --version
assert_strace_zero "--version: no epoll_create1" "epoll_create1" --version
assert_strace_zero "--version: no eventfd2" "eventfd2" --version

assert_strace_zero "doctor help: no clone3" "clone3" doctor help
assert_strace_zero "doctor help: no epoll_create1" "epoll_create1" doctor help
assert_strace_zero "doctor help: no eventfd2" "eventfd2" doctor help

summary
