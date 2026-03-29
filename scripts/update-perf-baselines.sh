#!/bin/bash
# Regenerate performance baselines from current benchmark measurements.
#
# Usage: scripts/update-perf-baselines.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
exec bash "$SCRIPT_DIR/perf-gate.sh" --update
