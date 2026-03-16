#!/usr/bin/env bash
# Help paths must not read secret files (keys, certs, credentials, vault).
source "$(dirname "$0")/lib.sh"
require_binary
require_strace "no sensitive file reads on help" || { summary; exit 0; }

assert_strace_absent "--help: no sensitive file reads" "openat" \
    '\.key"|\.pem"|credentials"|vault"|secrets"|\.env"' --help

summary
