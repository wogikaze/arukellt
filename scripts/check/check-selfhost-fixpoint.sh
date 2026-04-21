#!/usr/bin/env bash
# scripts/check/check-selfhost-fixpoint.sh
# Thin wrapper — delegates to python3 scripts/manager.py selfhost fixpoint.
# This wrapper exists so that documentation references using the legacy path continue to work.
#
# Usage:
#   bash scripts/check/check-selfhost-fixpoint.sh            # build + fixpoint
#   bash scripts/check/check-selfhost-fixpoint.sh --no-build # compare cached artifacts only
#   bash scripts/check/check-selfhost-fixpoint.sh --dry-run
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
exec python3 "$REPO_ROOT/scripts/manager.py" selfhost fixpoint "$@"
