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
**host** s2-runtime that performs the emit. Validate:

```bash
wasm-tools validate --features gc,function-references,memory64 \
  .build/selfhost/834-probe/selfhost-wasm32-gc-v17.wasm
```

Latest receipt: `selfhost-wasm32-gc-v17.wasm` — size 5 545 942 bytes,
sha256 `184d555483b3e359a995590b9b853a78697487b58f5ffd74aeece7a52bfaed52`, validate PASS.

## Phase 2 — P2 FS + host-linker (landed)

| Piece | Status |
|-------|--------|
| P2 imports `wasi:filesystem/types@0.2.0` `read`/`write` | landed |
| GC `fs_read` / `write_string` / `write_bytes` unstubbed | landed |
| `tools/host-linker` `p2_host.rs` (args, open-at, read, write, close, …) | landed |
| `checks.py` routes `wasi:cli/` / `wasi:filesystem/` via hosted runner | landed |
| Host argv: placeholder after prog so legacy `parse_args` index-1 command works | landed |
| GC initial memory pages = `initial_memory_pages()` (8192) for active data | landed |

### Smoke under host-linker (v17)

| Command | Result |
|---------|--------|
| `version` / `targets` | PASS |
| `fmt hello_min.ark` | PASS |
| `compile hello_min.ark` / `hello_probe.ark` → wasm32-gc/wasi-p2 | PASS (wrote output) |
| content probe `fs::read_to_string` + `starts_with` | PASS |
| write probe | PASS |
| **`compile src/compiler/main.ark` (self-compile / stdlib load)** | **FAIL** — trap in `finish_decimal_number` during `register_stdlib_source` |

## Phase 3 — GC emit correctness fixes (landed)

These unblocked parse/corehir for small programs:

1. **`len(p.errors)` / vec field len** — `find_call_source_local` preferred the struct base
   (`Parser`) over the field temp when no `LOCAL_GET` of the vec; fell back to `Vec<i32>`
   and `ref.cast` trapped. Fix: prefer vec-typed locals; resolve from preceding
   `STRUCT_GET`/`CALL` dest (`intrinsic_vec_type.ark`).
2. **Nested field `node.span.start`** — after outer `struct.get`, emitter reloaded an unset
   temp. Fix: when `prev` is `STRUCT_GET`/`GC_STRUCT_GET`/`CALL`, keep stack base
   (`inst_struct_record.ark`).
3. **`get_unchecked(...).field`** — same stack-base rule for preceding `CALL`.

Probes under `.build/selfhost/834-probe/`: `len_field_probe.ark`, `nested_field.ark`,
`get_field.ark`, `content_probe.ark` (not committed fixtures yet).

## Pin blockers (leave #834 open)

Do **not** pin / flip `BOOTSTRAP_EMIT_*` / drop #813 while the GC guest cannot
compile `src/compiler/main.ark` (stdlib lex). That would break `build-compiler`.

Remaining:

1. Fix GC lex path that traps in `finish_decimal_number` when loading stdlib
   (likely substring / decimal finish emit under GC).
2. Re-emit → host-linker self-compile main → validate.
3. Pin `bootstrap/arukellt-selfhost.wasm` (memory32 GC / wasi-p2; **do not**
   `--to-memory64` in `_ensure_bootstrap_compiler_wasm`).
4. Flip `BOOTSTRAP_EMIT_TARGET=wasm32-gc` / `BOOTSTRAP_EMIT_WASI_VERSION=wasi-p2`.
5. Drop #813 `_fixpoint_stage3_compiler` bootstrap-only path → s2-runtime.
6. `verify lane` then `verify quick`.

### Removal condition (for close)

All of:

- Flat-src emit+validate of `wasm32-gc`/`wasi-p2` selfhost (done).
- Host-linker P2 FS read/write usable for bootstrap (done for hello-scale).
- **GC guest can compile `src/compiler/main.ark`** (blocked — stdlib lex).
- `BOOTSTRAP_EMIT_*=wasm32-gc/wasi-p2` + drop #813 + `verify quick` green.

## Next actions

1. Root-cause `finish_decimal_number` GC emit (substring / float path).
2. Self-compile main under host-linker → pin → flip → drop #813 → verify.
