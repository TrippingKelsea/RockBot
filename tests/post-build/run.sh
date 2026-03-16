#!/usr/bin/env bash
# Post-build test runner — discovers and executes all perf_*.sh and sec_*.sh tests.
set -uo pipefail

DIR="$(cd "$(dirname "$0")" && pwd)"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

total_pass=0
total_fail=0
total_skip=0
failed_tests=()

echo "=== RockBot Post-Build Test Harness ==="
echo ""

# Discover test scripts
tests=()
for f in "$DIR"/perf_*.sh "$DIR"/sec_*.sh; do
    [ -f "$f" ] && tests+=("$f")
done

if [ ${#tests[@]} -eq 0 ]; then
    echo "No test scripts found."
    exit 1
fi

echo "Found ${#tests[@]} test(s)"
echo ""

for test_script in "${tests[@]}"; do
    name=$(basename "$test_script" .sh)
    echo "--- Running: $name ---"
    if bash "$test_script"; then
        total_pass=$((total_pass + 1))
    else
        total_fail=$((total_fail + 1))
        failed_tests+=("$name")
    fi
    echo ""
done

echo "=== Summary ==="
echo -e "${GREEN}${total_pass} passed${NC}, ${RED}${total_fail} failed${NC}"

if [ ${#failed_tests[@]} -gt 0 ]; then
    echo ""
    echo "Failed tests:"
    for t in "${failed_tests[@]}"; do
        echo -e "  ${RED}- ${t}${NC}"
    done
    exit 1
fi

exit 0
