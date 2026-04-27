#!/usr/bin/env bash
# update-target-status.sh — Update docs/target-contract.md from CI test results.
#
# USAGE
#   scripts/update-target-status.sh [OPTIONS] [INPUT_FILE]
#
# DESCRIPTION
#   Reads a JSON file describing per-target, per-surface status values produced by
#   CI target-behavior test runs, then updates the "Status" cells in the markdown
#   tables inside docs/target-contract.md.
#
#   Called by the CI drift-check job (issue #260) to keep the contract table in sync
#   with actual test results.  Safe to run locally — use --dry-run to preview changes.
#
# INPUT JSON FORMAT
#   The input file must be a JSON object keyed by Arukellt target identifier
#   (e.g. "wasm32-wasi-p1", "wasm32-wasi-p2").  Each value is an object mapping
#   surface names (matching the "Surface" column of the target's table in
#   docs/target-contract.md) to either:
#
#     • a status string: "guaranteed" | "smoke" | "scaffold" | "none" | "n/a"
#     • an object with a "status" key and optional "pass"/"fail" counts:
#         { "status": "guaranteed", "pass": 209, "fail": 0 }
#
#   Example:
#     {
#       "wasm32-wasi-p1": {
#         "parse":               { "status": "guaranteed", "pass": 209, "fail": 0 },
#         "typecheck":           "guaranteed",
#         "compile (core Wasm)": "guaranteed",
#         "run (wasmtime)":      { "status": "smoke", "pass": 205, "fail": 4 }
#       },
#       "wasm32-wasi-p2": {
#         "compile (core Wasm)": "guaranteed",
#         "run (wasmtime)":      "guaranteed"
#       }
#     }
#
#   Surface names are matched case-insensitively against the first column of each
#   table row.  Rows not mentioned in the input are left unchanged.
#
# OPTIONS
#   INPUT_FILE   Path to JSON input file.  Defaults to stdin if not provided.
#   --dry-run    Print proposed changes to stdout without modifying any file.
#   --contract   Path to target-contract.md (default: docs/target-contract.md
#                relative to repository root).
#   --help, -h   Print this usage message and exit 0.
#
# EXIT CODES
#   0  Success (or --dry-run with no changes needed)
#   1  Usage error or missing required tool
#   2  Input JSON parse error
#   3  docs/target-contract.md not found or not writable

set -euo pipefail

# ── Locate repository root ───────────────────────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# ── Defaults ─────────────────────────────────────────────────────────────────
DRY_RUN=0
INPUT_FILE=""
CONTRACT_FILE="${REPO_ROOT}/docs/target-contract.md"

# ── Usage ────────────────────────────────────────────────────────────────────
usage() {
  sed -n '/^# USAGE/,/^# EXIT CODES/{s/^# \{0,2\}//p}' "${BASH_SOURCE[0]}" \
    | sed '/^EXIT CODES$/q' \
    | head -n -1
}

# ── Parse arguments ───────────────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
  case "$1" in
    --help|-h)
      usage
      exit 0
      ;;
    --dry-run)
      DRY_RUN=1
      shift
      ;;
    --contract)
      shift
      CONTRACT_FILE="$1"
      shift
      ;;
    --contract=*)
      CONTRACT_FILE="${1#--contract=}"
      shift
      ;;
    -*)
      echo "error: unknown option: $1" >&2
      echo "       Run with --help for usage." >&2
      exit 1
      ;;
    *)
      if [[ -n "$INPUT_FILE" ]]; then
        echo "error: unexpected argument: $1" >&2
        exit 1
      fi
      INPUT_FILE="$1"
      shift
      ;;
  esac
done

# ── Preflight checks ─────────────────────────────────────────────────────────
if ! command -v python3 &>/dev/null; then
  echo "error: python3 is required but not found in PATH" >&2
  exit 1
fi

if [[ ! -f "$CONTRACT_FILE" ]]; then
  echo "error: target-contract.md not found: $CONTRACT_FILE" >&2
  exit 3
fi

if [[ "$DRY_RUN" -eq 0 && ! -w "$CONTRACT_FILE" ]]; then
  echo "error: target-contract.md is not writable: $CONTRACT_FILE" >&2
  exit 3
fi

# ── Read input JSON (file or stdin) ──────────────────────────────────────────
if [[ -n "$INPUT_FILE" ]]; then
  if [[ ! -f "$INPUT_FILE" ]]; then
    echo "error: input file not found: $INPUT_FILE" >&2
    exit 1
  fi
  INPUT_JSON="$(cat "$INPUT_FILE")"
else
  if [[ -t 0 ]]; then
    echo "error: no input file specified and stdin is a terminal" >&2
    echo "       Provide a JSON file as argument or pipe JSON to stdin." >&2
    echo "       Run with --help for usage." >&2
    exit 1
  fi
  INPUT_JSON="$(cat)"
fi

# ── Delegate to Python for markdown table update ──────────────────────────────
#    We export shell variables as environment variables so the heredoc python
#    script can read them without eval or complex quoting.
export _UTS_DRY_RUN="$DRY_RUN"
export _UTS_CONTRACT_FILE="$CONTRACT_FILE"
export _UTS_INPUT_JSON="$INPUT_JSON"

python3 <<'PYTHON'
import json
import os
import re
import sys
import difflib

# ── Read environment ─────────────────────────────────────────────────────────
dry_run       = os.environ["_UTS_DRY_RUN"] == "1"
contract_file = os.environ["_UTS_CONTRACT_FILE"]
input_json    = os.environ["_UTS_INPUT_JSON"]

# ── Parse JSON input ─────────────────────────────────────────────────────────
try:
    data = json.loads(input_json)
except json.JSONDecodeError as e:
    print(f"error: invalid JSON input: {e}", file=sys.stderr)
    sys.exit(2)

if not isinstance(data, dict):
    print("error: JSON input must be a top-level object keyed by target ID", file=sys.stderr)
    sys.exit(2)

# Normalise input: each surface value may be a string or {"status": ..., ...}
VALID_STATUSES = {"guaranteed", "smoke", "scaffold", "none", "n/a"}

def extract_status(v):
    if isinstance(v, str):
        return v
    if isinstance(v, dict) and "status" in v:
        return v["status"]
    return None

# Build a mapping: target_id -> { surface_lower -> new_status }
updates = {}
for target_id, surfaces in data.items():
    if not isinstance(surfaces, dict):
        print(f"warning: skipping target '{target_id}': expected object, got {type(surfaces).__name__}", file=sys.stderr)
        continue
    target_updates = {}
    for surface_name, value in surfaces.items():
        status = extract_status(value)
        if status is None:
            print(f"warning: skipping '{target_id}/{surface_name}': cannot extract status from {value!r}", file=sys.stderr)
            continue
        if status not in VALID_STATUSES:
            print(f"warning: skipping '{target_id}/{surface_name}': unrecognised status '{status}' (allowed: {', '.join(sorted(VALID_STATUSES))})", file=sys.stderr)
            continue
        target_updates[surface_name.strip().lower()] = status
    if target_updates:
        updates[target_id] = target_updates

if not updates:
    print("info: no valid updates in input JSON — nothing to do", file=sys.stderr)
    sys.exit(0)

# ── Read the contract document ───────────────────────────────────────────────
with open(contract_file, "r", encoding="utf-8") as fh:
    original_lines = fh.readlines()

lines = list(original_lines)

# ── Find section boundaries keyed by target identifier ───────────────────────
# Section headers look like:  ### T1 — `wasm32-wasi-p1` (CLI default)
SECTION_RE = re.compile(r"^###\s+\S+\s+.*?`([^`]+)`")
TABLE_ROW_RE = re.compile(r"^\|\s*(.+?)\s*\|\s*(.+?)\s*\|\s*(.+?)\s*\|")
TABLE_HEADER_RE = re.compile(r"^\|\s*Surface\s*\|\s*Status\s*\|\s*Detail\s*\|", re.IGNORECASE)

# Map target_id -> list of line indices that belong to that section's tables
section_start = {}   # target_id -> line index of section header
for i, line in enumerate(lines):
    m = SECTION_RE.match(line)
    if m:
        section_start[m.group(1)] = i

# ── Apply updates section by section ─────────────────────────────────────────
changed_lines = set()

for target_id, surface_map in updates.items():
    if target_id not in section_start:
        print(f"warning: no section found for target '{target_id}' in {contract_file}", file=sys.stderr)
        continue

    start = section_start[target_id]
    # Find end of section: next ### heading or EOF
    end = len(lines)
    for i in range(start + 1, len(lines)):
        if lines[i].startswith("### "):
            end = i
            break

    # Locate the Surface/Status/Detail table within the section
    in_table = False
    for i in range(start, end):
        stripped = lines[i].rstrip("\n")
        if TABLE_HEADER_RE.match(stripped):
            in_table = True
            continue
        if not in_table:
            continue
        # Separator row — skip
        if re.match(r"^\|[-| ]+\|$", stripped):
            continue
        # End of table block
        if not stripped.startswith("|"):
            in_table = False
            continue

        m = TABLE_ROW_RE.match(stripped)
        if not m:
            continue
        surface_cell  = m.group(1).strip()
        status_cell   = m.group(2).strip()
        detail_cell   = m.group(3).strip()

        surface_key = surface_cell.lower()
        if surface_key not in surface_map:
            continue

        new_status = surface_map[surface_key]
        if new_status == status_cell:
            continue  # already correct

        # Rebuild the row preserving the original column widths as much as possible
        # Original:  | parse | guaranteed | 209 `run` ... |
        # We need to replace exactly the status cell.
        old_row = lines[i]
        # Use simple replacement: replace first occurrence of the status cell between pipes
        # This handles the common "| surface | OLD_STATUS | detail |" pattern safely.
        new_row = re.sub(
            r"(\|\s*" + re.escape(surface_cell) + r"\s*\|)\s*" + re.escape(status_cell) + r"\s*(\|)",
            lambda mo: mo.group(1) + f" {new_status} " + mo.group(2),
            old_row,
            count=1,
        )
        if new_row != old_row:
            lines[i] = new_row
            changed_lines.add(i)

# ── Output ────────────────────────────────────────────────────────────────────
if not changed_lines:
    print("info: target-contract.md is already up to date — no changes needed")
    sys.exit(0)

if dry_run:
    diff = difflib.unified_diff(
        original_lines,
        lines,
        fromfile=f"a/{os.path.basename(contract_file)}",
        tofile=f"b/{os.path.basename(contract_file)}",
        lineterm="",
    )
    print("".join(diff))
    print(f"\n[dry-run] {len(changed_lines)} line(s) would be updated in {contract_file}")
else:
    with open(contract_file, "w", encoding="utf-8") as fh:
        fh.writelines(lines)
    print(f"updated {contract_file}: {len(changed_lines)} line(s) changed")
    for idx in sorted(changed_lines):
        print(f"  line {idx + 1}: {lines[idx].rstrip()}")

sys.exit(0)
PYTHON
