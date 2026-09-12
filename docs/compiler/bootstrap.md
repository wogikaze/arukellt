# Selfhost Bootstrap (ADR-029)

> **Current contract.** Trusted base is the pinned selfhost wasm. Rust-era
> Stage 0 narratives live in
> [`../history/reports/bootstrap-rust-era-compiler-guide.md`](../history/reports/bootstrap-rust-era-compiler-guide.md).

Normative decision: [`../adr/ADR-029-selfhost-native-verification-contract.md`](../adr/ADR-029-selfhost-native-verification-contract.md).  
Status summary: [`../state/compiler.md`](../state/compiler.md).  
Structured SSOT: [`../data/bootstrap-contract.toml`](../data/bootstrap-contract.toml).

## Which command?

Agents and humans confuse these two. Use the table:

| Goal | Command | Typical time | Notes |
|------|---------|--------------|-------|
| **Refresh the compiler after editing `src/compiler/**`** | `python3 scripts/manager.py selfhost build-compiler` | Depends on the current selfhost artifact | Stage-2 only. **Default for emitter / Memory64 / T3 work.** |
| Check ADR-029 fixpoint (`sha256(s2) == sha256(s3)`) | `python3 scripts/manager.py selfhost fixpoint` | seconds if s2/s3 exist | Does not refresh the emitter by itself |
| Rebuild s2 **and** s3 then compare (gate only) | `python3 scripts/manager.py selfhost fixpoint --build` | stage-3 alone can be **tens of minutes** today on gc+p2 | **Not** for routine iteration. gc-host ≤10s is [#851](../../issues/open/851-selfhost-compiler-core-rewrite.md) / [ADR-053](../adr/ADR-053-selfhost-compiler-core-rewrite.md) |

Aliases for `build-compiler`: `build-s2`, `rebuild-s2`.

Do **not** use `fixpoint --build --no-cache` to “just rebuild s2” — that also
runs stage-3 and floods long builds under parallel agents.

Copy files with `/bin/cp -f` (never interactive `cp -iv`).

### How to iterate without unnecessary rebuilds

`build-compiler` is a **full compile of the entire selfhost compiler**
(typecheck + MIR lower + wasm emit). Overlay cache hits only skip the flat-src
rewrite; they do **not** skip that compile.

**Do not rebuild once per one-line hypothesis.** That makes agents
latency-bound (`8s × N` tries).

Recommended loop:

1. Classify failures / read WAT with the **current** s2 (no rebuild).
2. Batch all planned `src/compiler/**` edits.
3. **One** `selfhost build-compiler`.
4. Re-validate **many** fixtures / the whole lane list against that s2.
5. Only rebuild again after the next batch of source edits.

Parallel agents must **share** one rebuilt s2 (parent rebuilds once); each lane
must not run its own `build-compiler`.

### wasm32 overlay compile: preopen the overlay only

The flat overlay copies a trimmed `std/prelude.ark` (it strips
`use std::collections::*`). Measuring or emitting a wasm32 compiler with
`--dir=<repo>` as well lets `std/prelude.ark` resolve to the full tree, which
pulls `impl String` methods. On wasm32 those methods rewrite `len`/`slice` and
the successor leaks (multi-GiB) during lower.

`scripts/selfhost/checks.py` `_wasm_compile` therefore preopens **only** the
overlay workspace when one is set. Manual timing must do the same:

```bash
wasmtime run --allow-precompiled --wasm gc --wasm function-references \
  -W memory64=y -W max-memory-size=17179869184 \
  --dir=.build/selfhost/flat-src \
  HOST.cwasm -- \
  compile src/compiler/main.ark --target wasm32 --wasi-version wasi-p1 \
  -o out.wasm
```

The bootstrap host emits a direct core module for the configured bootstrap
profile. A command component is packaged from standard P2 core imports with
the official `wasm-tools component embed` / `component new` commands; no
adapter or bridge is permitted.

## Trust model

| Stage | Artifact / check | Command |
|-------|------------------|---------|
| **0 (trust base)** | `bootstrap/arukellt-selfhost.wasm` (pinned; see `bootstrap/PROVENANCE.md`) | — |
| **Build current selfhost** | pinned compiles `src/compiler/main.ark` → `.build/selfhost/arukellt-s2.wasm` | `python3 scripts/manager.py selfhost build-compiler` |
| **Fixpoint** | `sha256(s2) == sha256(s3)` | `python3 scripts/manager.py selfhost fixpoint` |
| **Parity** | fixture / CLI / diag | `python3 scripts/manager.py selfhost fixture-parity`, `… parity --mode --cli`, `… diag-parity` |

Stage-3 / runtime compiler wasms are validated with `wasm-tools validate` after
build (and when reused). An invalid artifact — for example Memory64 GC output
that does `struct.set` of an i32 field without `i32.wrap_i64` — is deleted and
must not remain as a selectable `arukellt-s3.wasm`. The day-to-day wrapper
prefers `.build/selfhost/arukellt-s2-runtime.wasm` over s3 for this reason.

Stage 0 is **the pinned wasm**. The selfhost launcher has no Rust compiler
fallback. `ARUKELLT_USE_RUST=1` is rejected by the launcher.

## User-facing entrypoint

```bash
# Preferred: wrapper resolves pinned / s2 / env override
scripts/run/arukellt-selfhost.sh compile docs/examples/hello.ark --target wasm32-gc

# Or point ARUKELLT_SELFHOST_WASM at a freshly built s2 for library component work
ARUKELLT_SELFHOST_WASM=.build/selfhost/arukellt-s2.wasm \
  scripts/run/arukellt-selfhost.sh compile lib.ark --target wasm32-gc --emit component
```

Resolution order (wrapper `scripts/run/arukellt-selfhost.sh`, with each
existing candidate rejected if it uses a retired ABI):
`$ARUKELLT_SELFHOST_WASM` → `bootstrap/arukellt-selfhost.wasm` →
`.build/selfhost/arukellt-s2-runtime.wasm` → `.build/selfhost/arukellt-s3.wasm` →
`.build/selfhost/arukellt-s2.wasm` → `.bootstrap-build/arukellt-s2.wasm` →
`.build/selfhost/arukellt-pinned-bootstrap.wasm`.

## Artifact policy

| Path | Status |
|------|--------|
| Legacy P1-shaped P2 core imports | Rejected; no compatibility adapter |
| Repository-specific host imports | Rejected; use official WASI interfaces |
| Rust compiler fallback | Removed from the product and verification paths |

Historical walkthroughs of the old Rust→s1→s2 script remain under `docs/history/reports/`.

## CI

Bootstrap evidence runs in the **`selfhost`** job of `.github/workflows/ci.yml`
(fixpoint + parity). Do not invent a `verification-bootstrap` job name.
