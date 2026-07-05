#!/usr/bin/env bash
# scripts/perf/compare-linear-vs-gc.sh — Linear vs GC performance comparison wrapper.
#
# Compiles ADR-002 fixtures to both wasm32-wasi-p1 (linear) and wasm32-wasi-p2 (GC),
# then measures execution time across wasmtime, Node.js, and headless Chrome.
#
# Usage:
#   bash scripts/perf/compare-linear-vs-gc.sh [--iterations N] [--warmups N] \
#       [--runtimes wasmtime,node,browser] [--no-compile]
#
# Prerequisites:
#   - arukellt compiler in PATH
#   - wasmtime-py (pip install wasmtime)
#   - node (v22+ recommended for Wasm GC support)
#   - google-chrome (for browser runtime)
#   - puppeteer-core (cd scripts/perf && npm install)

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
exec python3 "${SCRIPT_DIR}/compare-linear-vs-gc.py" "$@"
