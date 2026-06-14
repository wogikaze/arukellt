#!/usr/bin/env bash
# Thin wrapper — canonical implementation lives in scripts/perf/.
exec bash "$(cd "$(dirname "$0")/.." && pwd)/perf/compare-benchmarks.sh" "$@"
