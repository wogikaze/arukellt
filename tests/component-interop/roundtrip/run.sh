#!/usr/bin/env bash
# WIT round-trip regression: emit Ark WIT and parse it with wasm-tools.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
cd "$REPO_ROOT"

if [[ -n "${ARUKELLT_BIN:-}" ]]; then
    ARUKELLT="$ARUKELLT_BIN"
else
    ARUKELLT="$REPO_ROOT/scripts/run/arukellt-selfhost.sh"
fi
WASM_TOOLS="${WASM_TOOLS_BIN:-$(command -v wasm-tools 2>/dev/null || true)}"

if [[ ! -f "$ARUKELLT" && ! -x "$ARUKELLT" ]]; then
    echo "SKIP: arukellt selfhost wrapper missing"
    exit 0
fi
if [[ -z "$WASM_TOOLS" ]]; then
    echo "SKIP: wasm-tools missing"
    exit 0
fi

OUT="$REPO_ROOT/.build/wit-roundtrip"
mkdir -p "$OUT"
export ARUKELLT_SELFHOST_WASM="${ARUKELLT_SELFHOST_WASM:-$REPO_ROOT/bootstrap/arukellt-selfhost.wasm}"

normalize_wit() {
    awk 'NF { sub(/[[:space:]]+$/, ""); print }' "$1"
}

PASS=0
FAIL=0
for scenario_dir in "$SCRIPT_DIR"/*/; do
    ark=""
    for candidate in "$scenario_dir"*.ark; do
        [[ -f "$candidate" ]] || continue
        ark="$candidate"
        break
    done
    [[ -n "$ark" ]] || continue

    name="$(basename "$ark" .ark)"
    golden="$scenario_dir${name}.expected.wit"
    emitted="$OUT/${name}.wit"
    if [[ ! -f "$golden" ]]; then
        echo "FAIL: $name missing golden WIT"
        FAIL=$((FAIL + 1))
        continue
    fi

    echo "== scenario: $name =="
    src_rel="${ark#$REPO_ROOT/}"
    out_rel=".build/wit-roundtrip/${name}.wit"
    if ! bash "$ARUKELLT" compile "$src_rel" --target wasm32-gc --emit wit -o "$out_rel"; then
        echo "FAIL: $name --emit wit failed"
        FAIL=$((FAIL + 1))
        continue
    fi
    if [[ ! -s "$emitted" ]]; then
        echo "FAIL: $name --emit wit produced no output"
        FAIL=$((FAIL + 1))
        continue
    fi
    if ! diff -u <(normalize_wit "$golden") <(normalize_wit "$emitted") >/dev/null; then
        echo "FAIL: $name --emit wit diverges from golden"
        diff -u <(normalize_wit "$golden") <(normalize_wit "$emitted") || true
        FAIL=$((FAIL + 1))
        continue
    fi
    "$WASM_TOOLS" component wit "$emitted" >/dev/null
    echo "  emit + wasm-tools parse: PASS"
    PASS=$((PASS + 1))
done

echo "WIT round-trip summary: PASS=$PASS FAIL=$FAIL"
if [[ "$FAIL" -gt 0 ]]; then
    exit 1
fi
if [[ "$PASS" -eq 0 ]]; then
    echo "SKIP: no scenarios executed"
    exit 0
fi
echo "PASS WIT round-trip"
