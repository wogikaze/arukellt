# Arukellt Release Criteria

> **Structured SSOT:** [`data/release-guarantees.toml`](data/release-guarantees.toml)
> (generated matrix: [`data/release-guarantees.md`](data/release-guarantees.md)).
> This prose view and [`release-checklist.md`](release-checklist.md) must stay aligned with that TOML.

This document defines what must be true before a tagged release.
"Guarantee" means: CI verifies this on every merge; a regression is a P1 bug.

## Pre-release Checklist

The exact guarantee commands, evidence scopes, CI jobs, and blocker status are
owned by [`data/release-guarantees.toml`](data/release-guarantees.toml) and its
generated [matrix](data/release-guarantees.md). Do not restate individual
guarantee procedures here. Run the aggregate release gates before a tag:

```bash
# Full verification (required before every tagged release)
python3 scripts/manager.py verify full
```

## Guarantee Tiers

### Guaranteed (stable)

Each guaranteed row in the structured catalogue states its exact evidence
scope. A passing smoke case guarantees only that declared scope; it is not an
unqualified proof for all programs, targets, profiles, or inputs.

### Provisional (may change)

These work but the API or behavior may change without a deprecation cycle:

- `--emit component` (works, but canonical ABI coverage is not exhaustive; living path may still use `wasm-tools` helpers)
- LSP hover/completion/diagnostics (functional, feature set evolving)
- `ark.toml` schema (fields continue to grow; `[world]` section is relatively new)

### Experimental (may break)

- `ark-dap` debug adapter (scaffold)
- VS Code extension DAP wiring (stub)

### Not guaranteed

- `wasm32-freestanding` as a public target（ADR-007: retired / hard error）
- `--target native-cpp` / `native-llvm`（scaffold。native-cppのprivate executor設計はADR-049、native-llvm ABIは未決定）
- `--target wasm32-gc --wasi p3`（host profile on primary language target; not started as a separate product）

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
