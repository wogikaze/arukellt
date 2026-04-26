#!/usr/bin/env bash
# check-diagnostic-codes.sh — verify error/warning codes match between implementation and docs
set -euo pipefail

CODES_RS="crates/ark-diagnostics/src/codes.rs"
ERROR_CODES_MD="docs/compiler/error-codes.md"

impl_codes=$(grep -oP '[EW]\d{4}' "$CODES_RS" | sort -u)
doc_codes=$(grep -oP '[EW]\d{4}' "$ERROR_CODES_MD" | sort -u)

missing_from_docs=$(comm -23 <(echo "$impl_codes") <(echo "$doc_codes"))
missing_from_impl=$(comm -13 <(echo "$impl_codes") <(echo "$doc_codes"))

ok=true

if [ -n "$missing_from_docs" ]; then
    echo "ERROR: codes in implementation but missing from docs:"
    echo "$missing_from_docs" | sed 's/^/  /'
    ok=false
fi

if [ -n "$missing_from_impl" ]; then
    echo "ERROR: codes in docs but missing from implementation:"
    echo "$missing_from_impl" | sed 's/^/  /'
    ok=false
fi

if $ok; then
    count=$(echo "$impl_codes" | wc -l)
    echo "OK: all $count diagnostic codes are aligned between implementation and docs"
fi

$ok
