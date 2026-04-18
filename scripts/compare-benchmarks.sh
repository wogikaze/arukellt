#!/usr/bin/env bash
# Cross-language benchmark comparison: native C / Rust / Go reference binaries
# vs Ark wasm under wasmtime. Delegates to scripts/run/run-benchmarks.sh.
#
# Usage:
#   bash scripts/compare-benchmarks.sh
#   bash scripts/compare-benchmarks.sh --quick
#   bash scripts/compare-benchmarks.sh --full
# Extra args are forwarded (e.g. --compare-lang overrides the default list).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
exec bash "$ROOT/scripts/run/run-benchmarks.sh" --compare-lang c,rust,go "$@"
