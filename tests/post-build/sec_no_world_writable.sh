#!/usr/bin/env bash
# Release binary must not have world-writable permissions.
source "$(dirname "$0")/lib.sh"
require_binary

perms=$(stat --format='%a' "$ROCKBOT_BIN" 2>/dev/null || stat -f '%Lp' "$ROCKBOT_BIN" 2>/dev/null)
other_write=$((perms % 10))
if [ $((other_write & 2)) -eq 0 ]; then
    pass "binary not world-writable (mode: ${perms})"
else
    fail "binary is world-writable (mode: ${perms})"
fi

summary
