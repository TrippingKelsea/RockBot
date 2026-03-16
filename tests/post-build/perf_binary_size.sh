#!/usr/bin/env bash
# Binary size budget for release build.
# Baseline: TBD (run and update after first measurement)
source "$(dirname "$0")/lib.sh"
require_binary

# Budget: 150MB (current baseline ~80-100MB)
assert_file_size_under "release binary size" "$ROCKBOT_BIN" 157286400

summary
