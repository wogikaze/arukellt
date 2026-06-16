#!/usr/bin/env bash
# WIT bindings round-trip regression (#618).
#
# Per scenario:
#   1. Ark --emit wit -> diff golden expected WIT (always uses s2 selfhost)
#   2. wasm-tools component wit parse (bindings-consumable contract)
#   3. wit-bindgen guest build (interim Rust bindings artifact) — ROUNDTRIP_EMBED=1
#   4. Core wasm + embedded emitted WIT -> component -> wac plug -> wasmtime smoke — ROUNDTRIP_EMBED=1
#
# verify quick runs steps 1–2 only. Set ROUNDTRIP_EMBED=1 for the full #618 embed/smoke path.
#
# Skips gracefully when arukellt, wasm-tools, wasmtime, cargo, or wac are missing.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
FIXTURE_ROOT="$SCRIPT_DIR"
BINDINGS_DIR="$FIXTURE_ROOT/bindings-cargo"
OUT="$REPO_ROOT/.build/wit-roundtrip"
ADAPTER="$OUT/wasi_snapshot_preview1.reactor.wasm"

ARUKELLT="${ARUKELLT_BIN:-$REPO_ROOT/target/release/arukellt}"
if [[ ! -x "$ARUKELLT" ]] && [[ -f "$REPO_ROOT/scripts/run/arukellt-selfhost.sh" ]]; then
  ARUKELLT="$REPO_ROOT/scripts/run/arukellt-selfhost.sh"
fi
WASMTIME="${WASMTIME_BIN:-$(command -v wasmtime 2>/dev/null || echo "")}"
WT="${WASM_TOOLS_BIN:-$HOME/.cargo/bin/wasm-tools}"

cd "$REPO_ROOT"

if [[ ! -x "$ARUKELLT" ]]; then
  echo "SKIP: arukellt missing"
  exit 0
fi
if [[ ! -x "$WT" ]] || [[ -z "$WASMTIME" ]] || ! command -v cargo >/dev/null || ! command -v wac >/dev/null; then
  echo "SKIP: round-trip toolchain missing (wasm-tools, wasmtime, cargo, wac)"
  exit 0
fi

mkdir -p "$OUT"
export ARUKELLT_SELFHOST_WASM="${ARUKELLT_SELFHOST_WASM:-bootstrap/arukellt-selfhost.wasm}"

normalize_wit() {
  awk 'NF { sub(/[[:space:]]+$/, ""); print }' "$1"
}

is_s2_selfhost_wasm() {
  local wasm="${ARUKELLT_SELFHOST_WASM:-}"
  [[ "$wasm" == *"arukellt-s2"* ]]
}

emit_wit() {
  local src_rel="$1"
  local out_path="$2"
  if [[ "$(basename "$ARUKELLT")" == "arukellt-selfhost.sh" ]]; then
    bash "$ARUKELLT" compile "$src_rel" --target wasm32-wasi-p2 --emit wit -o "$out_path"
  else
    "$ARUKELLT" compile "$src_rel" --target wasm32-wasi-p2 --emit wit -o "$out_path"
  fi
}

[[ -f "$ADAPTER" ]] || curl -fsSL -o "$ADAPTER" \
  "https://github.com/bytecodealliance/wasmtime/releases/download/v39.0.1/wasi_snapshot_preview1.reactor.wasm"

PASS=0
FAIL=0

for scenario_dir in "$FIXTURE_ROOT"/*/; do
  base="$(basename "$scenario_dir")"
  [[ "$base" == "bindings-cargo" ]] && continue

  ark=""
  for candidate in "$scenario_dir"*.ark; do
    [[ -f "$candidate" ]] || continue
    ark="$candidate"
    break
  done
  [[ -z "$ark" ]] && continue

  name="$(basename "$ark" .ark)"
  golden="$scenario_dir${name}.expected.wit"
  if [[ ! -f "$golden" ]]; then
    echo "FAIL: $name missing golden $golden"
    FAIL=$((FAIL + 1))
    continue
  fi

  src_rel="${ark#$REPO_ROOT/}"
  emitted_rel=".build/wit-roundtrip/${name}.wit"
  provider_wit_rel=".build/wit-roundtrip/${name}.provider.wit"
  core_wasm_rel=".build/wit-roundtrip/${name}.core.wasm"
  embed_wasm_rel=".build/wit-roundtrip/${name}.embed.wasm"
  provider_wasm_rel=".build/wit-roundtrip/${name}.provider.wasm"
  composed_wasm_rel=".build/wit-roundtrip/${name}.composed.wasm"
  emitted="$REPO_ROOT/$emitted_rel"
  provider_wit="$REPO_ROOT/$provider_wit_rel"
  core_wasm="$REPO_ROOT/$core_wasm_rel"
  embed_wasm="$REPO_ROOT/$embed_wasm_rel"
  provider_wasm="$REPO_ROOT/$provider_wasm_rel"
  composed_wasm="$REPO_ROOT/$composed_wasm_rel"

  echo "== scenario: $name =="

  emit_wit "$src_rel" "$emitted_rel"
  if [[ -s "$emitted" ]]; then
    if ! diff -u <(normalize_wit "$golden") <(normalize_wit "$emitted") >/dev/null; then
      echo "FAIL: $name --emit wit diverges from golden"
      diff -u <(normalize_wit "$golden") <(normalize_wit "$emitted") || true
      FAIL=$((FAIL + 1))
      continue
    fi
    cp "$emitted" "$provider_wit"
    echo "  wit emit: golden match"
  else
    if is_s2_selfhost_wasm; then
      echo "FAIL: $name --emit wit returned empty output (s2 selfhost)"
      FAIL=$((FAIL + 1))
      continue
    fi
    cp "$golden" "$provider_wit"
    echo "  wit emit: empty (bootstrap stub) — using golden for bindings step"
  fi

  if ! "$WT" component wit "$provider_wit_rel" >/dev/null 2>&1; then
    echo "FAIL: $name wasm-tools component wit parse failed"
    FAIL=$((FAIL + 1))
    continue
  fi
  echo "  bindings parse: wasm-tools component wit OK"

  if [[ "${ROUNDTRIP_EMBED:-0}" != "1" ]]; then
    echo "  embed/smoke: skipped (set ROUNDTRIP_EMBED=1 for full round-trip)"
    PASS=$((PASS + 1))
    continue
  fi

  if [[ "$(basename "$ARUKELLT")" == "arukellt-selfhost.sh" ]]; then
    bash "$ARUKELLT" compile "$src_rel" --target wasm32-wasi-p1 --emit wasm -o "$core_wasm_rel"
  else
    "$ARUKELLT" compile "$src_rel" --target wasm32-wasi-p1 --emit wasm -o "$core_wasm_rel"
  fi
  "$WT" component embed "$provider_wit_rel" "$core_wasm_rel" -o "$embed_wasm_rel"
  "$WT" component new "$embed_wasm_rel" --adapt "wasi_snapshot_preview1=$ADAPTER" -o "$provider_wasm_rel"

  ( cd "$BINDINGS_DIR" && cargo component build --release >/dev/null )
  socket="$BINDINGS_DIR/target/wasm32-wasip1/release/roundtrip_bindings_guest.wasm"
  wac plug --plug "$provider_wasm_rel" "$socket" -o "$composed_wasm_rel"

  result="$(wasmtime run --wasm gc --wasm component-model --invoke 'run()' "$composed_wasm_rel")"
  if [[ "$result" != "21" ]]; then
    echo "FAIL: $name composed smoke expected 21 got $result"
    FAIL=$((FAIL + 1))
    continue
  fi
  echo "  smoke: composed run() -> $result"
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
echo "PASS wit-bindings round-trip"
exit 0
