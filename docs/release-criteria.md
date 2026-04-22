# Arukellt Release Criteria

This document defines what must be true before each release tier.
"Guarantee" means: CI verifies this on every merge; a regression is a P1 bug.

## V1 Criteria (SATISFIED — closed 2026-03-27)

| Guarantee | Verification method |
|-----------|-------------------|
| T3 (`wasm32-wasi-p2`) compiles all v1 fixture set | `cargo test -p arukellt --test harness` |
| All values use Wasm GC heap | T3 type system + Wasm validator |
| T1 (`wasm32-wasi-p1`) still passes | Harness `run:` kind tests |
| No output-channel panics | ADR-015 audit + verify-harness.sh |

## V2 Criteria (SATISFIED — closed 2026-03-28)

| Guarantee | Verification method |
|-----------|-------------------|
| `--emit component` on `wasm32-wasi-p2` produces valid `.component.wasm` | Component fixture tests |
| `--emit wit` generates extractable WIT | WIT round-trip fixture |
| No regression in T1/T3 core Wasm | Full harness |

## V3 Criteria (SATISFIED — stdlib track complete)

| Guarantee | Verification method |
|-----------|-------------------|
| Stdlib module system (`use std::*`) functional | Fixture harness with manifest-driven tests |
| Scalar type completeness (all primitive types) | Type system validation + fixture coverage |
| Manifest-backed stdlib reference generated | `python3 scripts/gen/generate-docs.py` exit 0 |
| Prelude migration completed | Deprecated APIs flagged, migration guide exists |
| Stability labels applied to stdlib surface | `docs/stdlib-compatibility.md` verification |
| All v3 stdlib fixtures pass | `cargo test -p arukellt --test harness` (stdlib subset) |

## Pre-release Checklist

Run before any tagged release:

```bash
# 1. Full fixture harness
cargo test -p arukellt --test harness

# 2. Verification gate
python scripts/manager.py verify quick

# 3. Full verification (optional, before stable releases)
bash scripts/manager.py --full

# 4. Determinism check
arukellt compile docs/examples/hello.ark --target wasm32-wasi-p2 -o /tmp/h1.wasm
arukellt compile docs/examples/hello.ark --target wasm32-wasi-p2 -o /tmp/h2.wasm
sha256sum /tmp/h1.wasm /tmp/h2.wasm  # must match

# 5. Panic audit (included in verify-harness.sh --quick)
# no panic/unwrap in: crates/arukellt/src/, crates/ark-lsp/src/,
#   crates/ark-manifest/src/, crates/ark-driver/src/
```

## Guarantee Tiers

### Guaranteed (stable)

These things work and regressions are P1:

- `arukellt compile <file> --target wasm32-wasi-p2` produces valid Wasm
- `arukellt run <file>` executes the compiled output via wasmtime
- All 575 fixture tests pass
- Compilation is deterministic (same input → same output bytes)
- No panic on any user-reachable CLI path

### Provisional (may change)

These work but the API or behavior may change without a deprecation cycle:

- `--emit component` (works, but canonical ABI coverage is not exhaustive)
- LSP hover/completion/diagnostics (functional, feature set evolving)
- `ark.toml` schema (some fields added in v2; `[world]` section new)

### Experimental (may break)

- `--target wasm32-wasi-p1` (T1 supported, but not the primary CI gate)
- `ark-dap` debug adapter (scaffold)
- VS Code extension DAP wiring (stub)

### Not guaranteed

- `--target wasm32-freestanding` (T2: not started)
- `--target native` (T4: not implemented; ark-llvm scaffold removed in #586)
- `--target wasm32-wasi-p3` (T5: spec not finalized)

## Stability Policy

- `stable` features: no breaking changes without a major version bump + migration guide
- `provisional` features: may break with minor version bump + changelog entry
- `experimental` features: may break at any time; no migration guide required
- See [docs/stdlib-compatibility.md](stdlib-compatibility.md) for stdlib API policy

## References

- [docs/current-state.md](current-state.md) — current implementation status
- [docs/adr/ADR-013-primary-target.md](adr/ADR-013-primary-target.md) — tier system
- [docs/adr/ADR-014-stability-labels.md](adr/ADR-014-stability-labels.md) — label definitions
- [docs/adr/ADR-015-no-panic-in-user-paths.md](adr/ADR-015-no-panic-in-user-paths.md) — panic policy
