# Selfhost Bootstrap (ADR-029)

> **Current contract.** Trusted base is the pinned selfhost wasm, not a Rust
> compiler. Rust-era Stage 0 narratives live in
> [`../history/reports/bootstrap-rust-era-compiler-guide.md`](../history/reports/bootstrap-rust-era-compiler-guide.md).

Normative decision: [`../adr/ADR-029-selfhost-native-verification-contract.md`](../adr/ADR-029-selfhost-native-verification-contract.md).  
Status summary: [`../state/compiler.md`](../state/compiler.md).

## Trust model

| Stage | Artifact / check | Command |
|-------|------------------|---------|
| **0 (trust base)** | `bootstrap/arukellt-selfhost.wasm` (pinned; see `bootstrap/PROVENANCE.md`) | — |
| **Build current selfhost** | pinned compiles `src/compiler/main.ark` → `.build/selfhost/arukellt-s2.wasm` | `python3 scripts/manager.py selfhost fixpoint --build` |
| **Fixpoint** | `sha256(s2) == sha256(s3)` | same command / `selfhost fixpoint` |
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

Resolution order (wrapper): `$ARUKELLT_SELFHOST_WASM` → `.build/selfhost/arukellt-s3.wasm` →
`.build/selfhost/arukellt-s2.wasm` → … → `bootstrap/arukellt-selfhost.wasm`.

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
