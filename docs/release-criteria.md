# Arukellt Release Criteria

This document defines what must be true before a tagged release.
"Guarantee" means: CI verifies this on every merge; a regression is a P1 bug.

## Pre-release Checklist

Run before any tagged release:

```bash
# 1. Full fixture harness
python scripts/manager.py verify fixtures

# 2. Verification gate
python scripts/manager.py verify quick

# 3. Full verification (optional, before stable releases)
python scripts/manager.py verify --full

# 4. Determinism check
arukellt compile docs/examples/hello.ark --target wasm32-wasi-p2 -o /tmp/h1.wasm
arukellt compile docs/examples/hello.ark --target wasm32-wasi-p2 -o /tmp/h2.wasm
sha256sum /tmp/h1.wasm /tmp/h2.wasm  # must match

# 5. Panic audit (included in verify-harness.sh --quick)
# no legacy Rust CLI crate remains; the audit root list is intentionally empty
# until a shipped Rust entry point is introduced again.
# (the former Rust entrypoint and support packages were removed during #529)
```

## Guarantee Tiers

### Guaranteed (stable)

These things work and regressions are P1:

- `arukellt compile <file> --target wasm32-wasi-p2` produces valid Wasm
- `arukellt run <file>` executes the compiled output via wasmtime
- Fixture harness passes for the current manifest-backed set
- Compilation is deterministic (same input → same output bytes)
- No panic on any user-reachable CLI path

### Provisional (may change)

These work but the API or behavior may change without a deprecation cycle:

- `--emit component` (works, but canonical ABI coverage is not exhaustive)
- LSP hover/completion/diagnostics (functional, feature set evolving)
- `ark.toml` schema (fields continue to grow; `[world]` section is relatively new)

### Experimental (may break)

- `--target wasm32-wasi-p1` (T1 supported, but not the primary CI gate)
- `ark-dap` debug adapter (scaffold)
- VS Code extension DAP wiring (stub)

### Not guaranteed

- `--target wasm32-freestanding` (T2: scaffold)
- `--target native` (T4: scaffold; ark-llvm removed in #586)
- `--target wasm32-wasi-p3` (T5: not started)

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
