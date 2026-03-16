#!/usr/bin/env bash
# Wall-clock startup budget for --help.
# Baseline: TBD (run and update after first measurement)
source "$(dirname "$0")/lib.sh"
require_binary

# Budget: 50ms average over 10 iterations
assert_time_under_ms "--help startup time" 50 --help

summary
