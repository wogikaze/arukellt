#!/bin/bash
# Regenerate performance baselines from current measurements.
#
# Usage: scripts/update-perf-baselines.sh
#
# This runs the same benchmark suite as perf-gate.sh and writes
# the results as the new baseline file.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
exec bash "$SCRIPT_DIR/perf-gate.sh" --update
