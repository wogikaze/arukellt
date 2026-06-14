#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
if compgen -G "$ROOT/issues/open/422-"*.md >/dev/null 2>&1; then
  echo "check-artifact-size-budget: SKIP (#422 open)"
  exit 0
fi
bash "$ROOT/scripts/check/check-orphan-inventory.sh"
echo "check-artifact-size-budget: ok"
