#!/usr/bin/env bash
# Post-build test harness — shared helpers
# Source this file from individual test scripts.

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Counters
_PASS=0
_FAIL=0
_SKIP=0

# Binary path (override via ROCKBOT_BIN env var)
ROCKBOT_BIN="${ROCKBOT_BIN:-$(dirname "$0")/../../target/release/rockbot}"

pass() {
    _PASS=$((_PASS + 1))
    echo -e "${GREEN}PASS${NC}: $1"
}

fail() {
    _FAIL=$((_FAIL + 1))
    echo -e "${RED}FAIL${NC}: $1"
}

skip() {
    _SKIP=$((_SKIP + 1))
    echo -e "${YELLOW}SKIP${NC}: $1"
}

# Print summary and exit with appropriate code
summary() {
    echo ""
    echo "---"
    echo -e "Results: ${GREEN}${_PASS} passed${NC}, ${RED}${_FAIL} failed${NC}, ${YELLOW}${_SKIP} skipped${NC}"
    if [ "$_FAIL" -gt 0 ]; then
        exit 1
    fi
    exit 0
}

# Check that the release binary exists
require_binary() {
    if [ ! -x "$ROCKBOT_BIN" ]; then
        echo "ERROR: Release binary not found at $ROCKBOT_BIN"
        echo "Build with: cargo build --release"
        exit 2
    fi
}

# Skip test gracefully if strace is not available
require_strace() {
    if ! command -v strace &>/dev/null; then
        skip "$1 (strace not available)"
        return 1
    fi
    return 0
}

# Assert that a syscall matching PATTERN has 0 calls in strace -c output.
# Usage: assert_strace_zero "description" PATTERN ARGS...
assert_strace_zero() {
    local desc="$1"; shift
    local pattern="$1"; shift
    local output
    output=$(strace -c "$ROCKBOT_BIN" "$@" 2>&1 >/dev/null || true)
    # strace -c prints a table; grep for the syscall name
    # Lines look like: "  0.00    0.000000           0         0 clone3"
    # We check if the syscall appears with a non-zero call count
    if echo "$output" | grep -qE "[1-9][0-9]* +[0-9]+ +${pattern}"; then
        fail "$desc (found calls to ${pattern})"
        echo "$output" | grep -E "${pattern}" || true
        return 1
    else
        pass "$desc"
        return 0
    fi
}

# Assert that GREP_PATTERN is absent in strace trace output.
# Usage: assert_strace_absent "description" TRACE_FILTER GREP_PATTERN ARGS...
assert_strace_absent() {
    local desc="$1"; shift
    local trace_filter="$1"; shift
    local grep_pattern="$1"; shift
    local output
    output=$(strace -e "trace=${trace_filter}" "$ROCKBOT_BIN" "$@" 2>&1 >/dev/null || true)
    if echo "$output" | grep -qE "$grep_pattern"; then
        fail "$desc (found: ${grep_pattern})"
        echo "$output" | grep -E "$grep_pattern" | head -5
        return 1
    else
        pass "$desc"
        return 0
    fi
}

# Assert total syscall count is under MAX.
# Usage: assert_strace_max "description" MAX ARGS...
assert_strace_max() {
    local desc="$1"; shift
    local max="$1"; shift
    local output
    output=$(strace -c "$ROCKBOT_BIN" "$@" 2>&1 >/dev/null || true)
    # The "total" line looks like: "100.00    0.001234                     423           7 total"
    local total
    total=$(echo "$output" | grep -E '^\s*100\.00' | awk '{for(i=1;i<=NF;i++) if($i ~ /^[0-9]+$/) {print $i; exit}}')
    if [ -z "$total" ]; then
        # Fallback: sum all syscall counts
        total=$(echo "$output" | tail -n +4 | head -n -2 | awk '{s+=$4} END{print s+0}')
    fi
    if [ "$total" -le "$max" ]; then
        pass "$desc (${total} syscalls, budget: ${max})"
    else
        fail "$desc (${total} syscalls, budget: ${max})"
    fi
}

# Assert average wall-clock time is under MAX_MS milliseconds.
# Usage: assert_time_under_ms "description" MAX_MS ARGS...
assert_time_under_ms() {
    local desc="$1"; shift
    local max_ms="$1"; shift
    local iterations=10
    local total_ns=0
    for _ in $(seq 1 $iterations); do
        local start end elapsed
        start=$(date +%s%N)
        "$ROCKBOT_BIN" "$@" >/dev/null 2>&1 || true
        end=$(date +%s%N)
        elapsed=$((end - start))
        total_ns=$((total_ns + elapsed))
    done
    local avg_ms=$((total_ns / iterations / 1000000))
    if [ "$avg_ms" -le "$max_ms" ]; then
        pass "$desc (${avg_ms}ms avg, budget: ${max_ms}ms)"
    else
        fail "$desc (${avg_ms}ms avg, budget: ${max_ms}ms)"
    fi
}

# Assert file size is under MAX_BYTES.
# Usage: assert_file_size_under "description" PATH MAX_BYTES
assert_file_size_under() {
    local desc="$1"; shift
    local filepath="$1"; shift
    local max_bytes="$1"; shift
    if [ ! -f "$filepath" ]; then
        fail "$desc (file not found: ${filepath})"
        return 1
    fi
    local size
    size=$(stat --format='%s' "$filepath" 2>/dev/null || stat -f '%z' "$filepath" 2>/dev/null)
    local size_mb=$((size / 1024 / 1024))
    local max_mb=$((max_bytes / 1024 / 1024))
    if [ "$size" -le "$max_bytes" ]; then
        pass "$desc (${size_mb}MB, budget: ${max_mb}MB)"
    else
        fail "$desc (${size_mb}MB, budget: ${max_mb}MB)"
    fi
}
