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
| **Refresh the compiler after editing `src/compiler/**`** | `python3 scripts/manager.py selfhost build-compiler` | **~45–50s** (warm overlay) | Stage-2 only. **Default for emitter / Memory64 / T3 work.** |
| Check ADR-029 fixpoint (`sha256(s2) == sha256(s3)`) | `python3 scripts/manager.py selfhost fixpoint` | seconds if s2/s3 exist | Does not refresh the emitter by itself |
| Rebuild s2 **and** s3 then compare (gate only) | `python3 scripts/manager.py selfhost fixpoint --build` | several minutes | **Not** for routine iteration |

Aliases for `build-compiler`: `build-s2`, `rebuild-s2`.

Do **not** use `fixpoint --build --no-cache` to “just rebuild s2” — that also
runs stage-3, floods long builds, and has caused Connection stalled under
parallel agents.

Copy files with `/bin/cp -f` (never interactive `cp -iv`).

### Why ~45s, and how to iterate without dying

`build-compiler` is a **full pinned→s2 compile of the entire selfhost compiler**
(typecheck + MIR lower + wasm emit). Overlay cache hits only skip the flat-src
rewrite (~0.1s); they do **not** skip that compile. That ~45s is the practical
floor today — not a fixpoint/stage-3 tax.

**Do not rebuild once per one-line hypothesis.** That makes agents
latency-bound (`45s × N` tries).

Recommended loop:

1. Classify failures / read WAT with the **current** s2 (no rebuild).
2. Batch all planned `src/compiler/**` edits.
3. **One** `selfhost build-compiler`.
4. Re-validate **many** fixtures / the whole lane list against that s2.
5. Only rebuild again after the next batch of source edits.

Parallel agents must **share** one rebuilt s2 (parent rebuilds once); each lane
must not run its own `build-compiler`.

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
