#!/usr/bin/env bash
# Wrapper around the CI-verified compose smoke test.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
exec bash "$ROOT/tests/component-interop/compose/run.sh"
