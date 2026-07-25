# #834 Probe: wasm32-gc self-emit on 23GiB host (2026-07-26)

## Host

- MemTotal ≈ 23 GiB, Swap 6 GiB (WSL2)
- Lane: `ARUKELLT_BUILD_DIR=$PWD/.build` under `.worktrees/wave-834-bootstrap`

## Phase 1 — emit without OOM (morning)

Default Memory64 s2-runtime (`initial_pages=65535`):

| Attempt | Target | WASI | Wall | Max RSS | Result |
|---------|--------|------|------|---------|--------|
| default | wasm32-gc | wasi-p2 | 201.7s → later 61s cached | ~1.2–1.3 GiB | emit OK |
| m64 | wasm32-gc | wasi-p1 | 141.2s | ~1.2 GiB | emit OK |

## Phase 2 — false “hang” diagnosis (afternoon)

Wrong invoke shapes looked like hang/OOM:

| Invoke | Peak RSS | Result |
|--------|----------|--------|
| `--dir=. --dir=src::src --dir=std::std --dir=.build` | ~6 GiB flat CPU | timeout / killed (not progress) |
| single `--dir=.` / `_wasm_compile` only | grows to 4 GiB | trap `OOB 0x100090000` |
| **flat-src success path (below)** | **~1.23 GiB** | **emit OK ~100s** |

**Not hard OOM** on this host when using the bootstrap flat-src preopen layout.
The ~6 GiB “hang” is a bad dir/preopen shape (and/or missing `--cache-dir`), not Memory64
exhaustion. clean `HEAD` reproduced the bad path; it does **not** prove the typing patch
increased RSS.

## Working emit command (measured)

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
rm -f .build/selfhost/flat-src/bootstrap-out.wasm
/usr/bin/time -f 'WALL_SEC=%e MAX_RSS_KB=%M EXIT=%x' \
  "$HOME/.wasmtime/bin/wasmtime" run \
    --wasm gc --wasm function-references \
    -W memory64=y -W max-memory-size=17179869184 \
    --dir=.build/selfhost/flat-src --dir=. \
    .build/selfhost/arukellt-s2-runtime.wasm -- \
    compile src/compiler/main.ark --target wasm32-gc --wasi-version wasi-p2 \
    -o bootstrap-out.wasm --cache-dir .build/selfhost/ast-cache
```

| Receipt | Value |
|---------|-------|
| WALL_SEC | 97.75 (v6) / 102.22 (v7 after `init.ark` fix) |
| MAX_RSS_KB | 1230696 / 1230304 (~1.17 GiB) |
| EXIT | 0 |
| output | `.build/selfhost/flat-src/bootstrap-out.wasm` (~5.5 MiB) |
| copy | `.build/selfhost/834-probe/selfhost-wasm32-gc-v7.wasm` |

Runtime memory section (host s2): Memory64, `initial_pages=65535`, no max
(same as morning success). Variants with `--initial-pages=8192/16384/32768` were built
under `.build/selfhost/834-probe/runtimes/` but **not required** once flat-src was used.

## Phase 2 — validate

1. Import-index fix (#834 earlier): wasi-p2 stdin no longer calls `open-at`.
2. Result-local typing + typed GC fs stubs: fixtures `write_init` / `read_init` validate.
3. Full module validate **failed** on `cmd_init` while `init.ark` called
   `fs::fs_error_message(e)` with `e: String` from `Result<(), String>` (expects `FsError`).
   Emitter lowered that arm to `unreachable` + ill-typed dead concat (`expected ref, found i32`).
4. **Fix:** `src/compiler/main/init.ark` prints `e` directly.
5. **v7 validate:** `wasm-tools validate --features gc,function-references,memory64` → **PASS**.

```bash
wasm-tools validate --features gc,function-references,memory64 \
  .build/selfhost/834-probe/selfhost-wasm32-gc-v7.wasm
```

## Pin blockers (leave #834 open)

Acceptance still needs a **runnable** pinned bootstrap, not only a validating blob:

1. Artifact imports `wasi:cli/stdout@0.2.0` etc. Plain `wasmtime run` without the P2/component
   host fails: `unknown import get-stdout` (smoke, 2026-07-26).
2. GC `fs::read_to_string` remains a typed Err stub — pin cannot read sources.
3. P2 core surface has **no file `fd_write`**; GC `write_string` / `write_bytes` are typed Ok
   stubs under wasi-p2. Pin cannot write outputs.
4. P2 `fd_read` import slot is `stdin.read`, not a filesystem read — unstubbing read onto
   that index is not sufficient for source loads.

### Removal condition (for close)

All of:

- Stable flat-src (or equivalent) emit+validate of `wasm32-gc`/`wasi-p2` selfhost (this probe).
- Real GC filesystem read (and write) on the P2 host surface used by bootstrap run,
  **or** an accepted ADR change of `BOOTSTRAP_EMIT_*` away from wasi-p2.
- `BOOTSTRAP_EMIT_TARGET=wasm32-gc` / `BOOTSTRAP_EMIT_WASI_VERSION=wasi-p2` (or successor).
- Drop #813 stage-3 bootstrap-only workaround.
- `python3 scripts/manager.py verify quick` 0 failures.

## Next actions

1. Wire P2 filesystem read/write host imports (or decide emit WASI for pin).
2. Un-stub GC `fs_read` / `write_string` against that surface.
3. Re-emit → validate → pin → flip `BOOTSTRAP_EMIT_*` → drop #813 → `verify quick`.
