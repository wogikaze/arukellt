# #834 Probe: wasm32-gc self-emit on 23GiB host (2026-07-26)

## Host

- MemTotal ≈ 23 GiB, Swap 6 GiB (WSL2)
- Lane: `ARUKELLT_BUILD_DIR=$PWD/.build` under `.worktrees/wave-834-bootstrap`
- Branch: `wave/834-bootstrap`

## Phase 1 — emit without OOM

Default Memory64 s2-runtime (`initial_pages=65535`):

| Attempt | Target | WASI | Wall | Max RSS | Result |
|---------|--------|------|------|---------|--------|
| default | wasm32-gc | wasi-p2 | ~60–130s cached | ~1.23 GiB | emit OK |
| flat-src | wasm32-gc | wasi-p2 | ~100s | ~1.23 GiB | emit OK |

## Working emit command (flat-src)

```bash
cd .worktrees/wave-834-bootstrap
export ARUKELLT_BUILD_DIR="$PWD/.build"
python3 - <<'PY'
import sys
from pathlib import Path
sys.path.insert(0, "scripts")
from selfhost import checks
print(checks._prepare_flattened_selfhost_source(Path(".").resolve()))
PY
"$HOME/.wasmtime/bin/wasmtime" run \
  --wasm gc --wasm function-references \
  -W memory64=y -W max-memory-size=17179869184 \
  --dir=.build/selfhost/flat-src --dir=. \
  .build/selfhost/arukellt-s2-runtime.wasm -- \
  compile src/compiler/main.ark --target wasm32-gc --wasi-version wasi-p2 \
  -o bootstrap-out.wasm --cache-dir .build/selfhost/ast-cache
```

Guest memory is **memory32** (`(memory 8192)`), not Memory64. Memory64 applies to the
**host** s2-runtime that performs the emit (until the pin itself is GC). Validate:

```bash
wasm-tools validate --features gc,function-references,memory64 \
  .build/selfhost/834-probe/selfhost-wasm32-gc-v19.wasm
```

## Phase 2 — P2 FS + host-linker (landed)

| Piece | Status |
|-------|--------|
| P2 imports `wasi:filesystem/types@0.2.0` `read`/`write` | landed |
| GC `fs_read` / `write_string` / `write_bytes` unstubbed | landed |
| `tools/host-linker` `p2_host.rs` (args, open-at, read, write, close, …) | landed |
| `checks.py` routes `wasi:cli/` / `wasi:filesystem/` via hosted runner | landed |
| Host argv: placeholder after prog so legacy `parse_args` index-1 command works | landed |
| GC initial memory pages = `initial_memory_pages()` (8192) for active data | landed |

## Phase 3 — GC emit correctness fixes (landed)

1. **`len(p.errors)` / vec field len** — `intrinsic_vec_type.ark`
2. **Nested field `node.span.start` / `get_unchecked(...).field`** — `inst_struct_record.ark`
3. **`parse_f64` GC stub → real emit** — `intrinsic_parse_f64_gc.ark`: copy String to
   linear, reuse LM digit scanner, wrap `Result::Ok(_f1_f64)` / `Err(String)`.
   Prior stub emitted `ref.null` Result and trapped in `finish_decimal_number` match.
4. **`mir_int_literal_needs_i64` OOB** — `literal_int.ark`: guard every `char_at(..., 1)`
   behind `raw_len >= 2` (flat `a && b || c` let OR arms run when `a` is false; GC
   `array.get_u` traps on one-digit literals).

## Phase 4 — self-compile + pin (landed)

| Check | Result |
|-------|--------|
| host-linker `compile float_only.ark` (v19) | PASS |
| host-linker flat-src `compile src/compiler/main.ark` (v19) | PASS (~133s, ~1.4 GiB RSS) |
| `sha256(v19) == sha256(selfhost-from-v19)` | PASS `4d2da710…` |
| `wasm-tools validate` v19 | PASS |
| Pin `bootstrap/arukellt-selfhost.wasm` | memory32 GC / wasi-p2 (no `--to-memory64`) |
| `BOOTSTRAP_EMIT_*=wasm32-gc/wasi-p2` | flipped |
| #813 stage-3 bootstrap-only path | dropped → s2-runtime |

Pinned fixpoint: `bootstrap/arukellt-selfhost.wasm` — size 5 553 192 bytes,
sha256 `4d2da710115215965514608fe8f1d70cedabba35adf1c729abb0c0d2aa7539bd`
(`sha256(pin)==sha256(s2)==sha256(s3)` after GC bootstrap chain).

## References

- [#834](../../issues/done/834-wasm32-gc-bootstrap-pin.md) (closed)
- `bootstrap/PROVENANCE.md`
