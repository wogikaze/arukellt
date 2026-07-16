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
| **Refresh the compiler after editing `src/compiler/**`** | `python3 scripts/manager.py selfhost build-compiler` | ~50s (warm overlay cache) | Stage-2 only. **Default for emitter / Memory64 / T3 work.** |
| Check ADR-029 fixpoint (`sha256(s2) == sha256(s3)`) | `python3 scripts/manager.py selfhost fixpoint` | seconds if s2/s3 exist | Does not refresh the emitter by itself |
| Rebuild s2 **and** s3 then compare (gate only) | `python3 scripts/manager.py selfhost fixpoint --build` | several minutes | **Not** for routine iteration |

Aliases for `build-compiler`: `build-s2`, `rebuild-s2`.

Do **not** use `fixpoint --build --no-cache` to “just rebuild s2” — that also
runs stage-3, floods long builds, and has caused Connection stalled under
parallel agents.

Copy files with `/bin/cp -f` (never interactive `cp -iv`).

## Trust model

| Stage | Artifact / check | Command |
|-------|------------------|---------|
| **0 (trust base)** | `bootstrap/arukellt-selfhost.wasm` (pinned; see `bootstrap/PROVENANCE.md`) | — |
| **Build current selfhost** | pinned compiles `src/compiler/main.ark` → `.build/selfhost/arukellt-s2.wasm` | `python3 scripts/manager.py selfhost build-compiler` |
| **Fixpoint** | `sha256(s2) == sha256(s3)` | `python3 scripts/manager.py selfhost fixpoint` |
| **Parity** | fixture / CLI / diag | `python3 scripts/manager.py selfhost fixture-parity`, `… parity --mode --cli`, `… diag-parity` |

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
