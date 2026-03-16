#!/usr/bin/env bash
# Help paths must not exec child processes (only 1 execve: the binary itself).
source "$(dirname "$0")/lib.sh"
require_binary
require_strace "no child execs on help" || { summary; exit 0; }

output=$(strace -e trace=execve "$ROCKBOT_BIN" --help 2>&1 >/dev/null || true)
execve_count=$(echo "$output" | grep -c 'execve(' || true)

if [ "$execve_count" -le 1 ]; then
    pass "--help: no child process spawning (${execve_count} execve)"
else
    fail "--help: spawned child processes (${execve_count} execve calls)"
    echo "$output" | grep 'execve(' | head -5
fi

summary
