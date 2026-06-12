#!/usr/bin/env bash
# Orphan / stale file inventory (advisory — always exits 0).
# Issue #418 — scans docs/tests/benchmarks/artifacts categories.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../" && pwd)"
exec python3 "$ROOT/scripts/check/check-orphan-inventory.py" "$@"
