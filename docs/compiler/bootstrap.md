# Selfhost Bootstrap (ADR-029)

> **Current contract.** Trusted base is the pinned selfhost wasm, not a Rust
> compiler. Rust-era Stage 0 narratives live in
> [`../history/reports/bootstrap-rust-era-compiler-guide.md`](../history/reports/bootstrap-rust-era-compiler-guide.md).

Normative decision: [`../adr/ADR-029-selfhost-native-verification-contract.md`](../adr/ADR-029-selfhost-native-verification-contract.md).  
Status summary: [`../state/compiler.md`](../state/compiler.md).  
Structured SSOT: [`../data/bootstrap-contract.toml`](../data/bootstrap-contract.toml).

## Which command?

Agents and humans confuse these two. Use the table:

| Goal | Command | Typical time | Notes |
|------|---------|--------------|-------|
| **Refresh the compiler after editing `src/compiler/**`** | `python3 scripts/manager.py selfhost build-compiler` | **~8–10s** (warm wasm32 overlay + AOT runtime) | Stage-2 only. **Default for emitter / Memory64 / T3 work.** |
| Check ADR-029 fixpoint (`sha256(s2) == sha256(s3)`) | `python3 scripts/manager.py selfhost fixpoint` | seconds if s2/s3 exist | Does not refresh the emitter by itself |
| Rebuild s2 **and** s3 then compare (gate only) | `python3 scripts/manager.py selfhost fixpoint --build` | stage-3 alone can be **tens of minutes** today on gc+p2 | **Not** for routine iteration. gc-host ≤10s is [#851](../../issues/open/851-selfhost-compiler-core-rewrite.md) / [ADR-053](../adr/ADR-053-selfhost-compiler-core-rewrite.md) |

Aliases for `build-compiler`: `build-s2`, `rebuild-s2`.

Do **not** use `fixpoint --build --no-cache` to “just rebuild s2” — that also
runs stage-3, floods long builds, and has caused Connection stalled under
parallel agents.

Copy files with `/bin/cp -f` (never interactive `cp -iv`).

### Why ~8–10s, and how to iterate without dying

`build-compiler` is a **full compile of the entire selfhost compiler**
(typecheck + MIR lower + wasm emit). Overlay cache hits only skip the flat-src
rewrite (~0.1s); they do **not** skip that compile. With official
`BOOTSTRAP_EMIT_TARGET=wasm32` and an AOT Memory64 runtime host, the no-cache
floor on this machine is **~8–10s** — not a fixpoint/stage-3 tax.

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

On this machine that path is no-cache **~8.5s** median (`sha256` matches a
second emit from the same host). Official `BOOTSTRAP_EMIT_TARGET` is `wasm32`
/ `wasi-p1`. The pinned trust base stays `wasm32-gc` / `wasi-p2` (#834) and
only hops through a current-source gc compiler when no wasm32 runtime exists.

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

Stage 0 is **the pinned wasm**. There is no Rust Stage 0. Setting
`ARUKELLT_USE_RUST=1` hard-fails in [`scripts/run/arukellt-selfhost.sh`](../../scripts/run/arukellt-selfhost.sh).

## User-facing entrypoint

```bash
# Preferred: wrapper resolves pinned / s2 / env override
scripts/run/arukellt-selfhost.sh compile docs/examples/hello.ark --target wasm32-gc

# Or point ARUKELLT_SELFHOST_WASM at a freshly built s2 for library component work
ARUKELLT_SELFHOST_WASM=.build/selfhost/arukellt-s2.wasm \
  scripts/run/arukellt-selfhost.sh compile lib.ark --target wasm32-gc --emit component
```

Resolution order (wrapper `scripts/run/arukellt-selfhost.sh`):
`$ARUKELLT_SELFHOST_WASM` → `.build/selfhost/arukellt-s3.wasm` →
`.build/selfhost/arukellt-s2-runtime.wasm` → `.build/selfhost/arukellt-s2.wasm` →
`.bootstrap-build/arukellt-s2.wasm` → `.build/selfhost/arukellt-pinned-bootstrap.wasm` →
`bootstrap/arukellt-selfhost.wasm`.

## Retired paths

| Path | Status |
|------|--------|
| `scripts/run/verify-bootstrap.sh` Rust Stage 0 | **Retired** for current selfhost source surface (see release-checklist deferred note) |
| `ARUKELLT_USE_RUST=1` | Hard error (#583 / ADR-029) |
| Comparing against `target/debug/arukellt` Rust binary as trust base | Not part of the current contract |

Historical walkthroughs of the old Rust→s1→s2 script remain under `docs/history/reports/`.

## CI

Bootstrap evidence runs in the **`selfhost`** job of `.github/workflows/ci.yml`
(fixpoint + parity). Do not invent a `verification-bootstrap` job name.
