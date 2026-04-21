# Bootstrap Verification

This document is the **process-level** entry for the staged bootstrap pipeline
and fixpoint gate.  The executable contract lives in
`scripts/run/verify-bootstrap.sh`; the user-facing compiler guide is
[`docs/compiler/bootstrap.md`](../compiler/bootstrap.md).

**Tracking:** scaffold and naming contract for issue

## 154 (`issues/open/154-bootstrap-verification-scaffold.md`); roadmap alignment

`docs/process/roadmap-cross-cutting.md` §6.5, `docs/process/roadmap-v5.md`.

## Goal

Prove that the self-hosted compiler is correct by reaching a **fixpoint**:
compiling the compiler with itself produces a **byte-identical** wasm binary
when the trusted base (Rust compiler) and the first self-compile agree.

## Stages (canonical slots)

These map 1:1 to the implementation in `scripts/run/verify-bootstrap.sh`.

| Stage | Input | Tool | Output | Role |
|-------|--------|------|--------|------|
| **0** | `src/compiler/main.ark` | Rust `arukellt` (`ARUKELLT_BIN` or `target/{debug,release}/arukellt`) | `.bootstrap-build/arukellt-s1.wasm` | Trusted base |
| **1** | same `main.ark` (paths relative to repo root) | `wasmtime run` of `arukellt-s1.wasm` | `.bootstrap-build/arukellt-s2.wasm` | First self-compile |
| **2** | `arukellt-s1.wasm`, `arukellt-s2.wasm` | `sha256sum` | (none) | Fixpoint: digests must match |

Intermediate paths live under **`.bootstrap-build/`** and are removed when the
script exits (cleanup trap).  Do not treat that directory as a cache across
runs unless you copy artifacts out first.

## Artifact naming convention

| Artifact | Path | Producer |
|----------|------|----------|
| `arukellt-s1.wasm` | `.bootstrap-build/arukellt-s1.wasm` | Stage 0 (Rust compiler) |
| `arukellt-s2.wasm` | `.bootstrap-build/arukellt-s2.wasm` | Stage 1 (self-compile) |

Per-stage compiler stderr is captured to `.bootstrap-build/stage0.stderr` and
`.bootstrap-build/stage1.stderr` when present; the script may indent and print
these on failure.

## Failure and diff policy

- Any failed stage exits **1** for the full gate; partial modes (`--stage1-only`,
  `--stage N`, `--fixture-parity`) exit **0** only when the requested subset
  succeeds and **do not** claim fixpoint attainment unless the full 0→1→2 path
  ran successfully.
- **Stage 2** does not run `diff` on wasm bytes.  On mismatch it prints both
  **sha256** digests and **file sizes**, then points to
  `scripts/run/compare-outputs.sh` to locate the first divergent **phase** on a
  fixture (see `docs/compiler/bootstrap.md` — Debug Procedures).
- **`--check`:** machine-readable summary for the full gate only; incompatible
  with partial flags (see `docs/compiler/bootstrap.md` — Completion contract).

## Determinism requirement

Fixpoint verification requires **deterministic** compilation: no timestamps,
random nonces, or pointer-derived bytes in the wasm output.  Harness
determinism checks for fixtures are extended toward selfhost over time; see
`docs/compiler/bootstrap.md` and `scripts/run/verify-harness.sh`.

## Running

```bash
# Full Stage 0 → 1 → 2 (fixpoint attainment gate when all pass)
scripts/run/verify-bootstrap.sh

# Stage 0 only (smoke; does not prove fixpoint)
scripts/run/verify-bootstrap.sh --stage1-only

# Single stage
scripts/run/verify-bootstrap.sh --stage 0
```

## Integration with verify-harness

`scripts/run/verify-harness.sh` is the top-level completion gate.

- **Today:** optional `--fixpoint` runs `scripts/check/check-selfhost-fixpoint.sh`
  (issue #459; may SKIP or soft-fail depending on artifacts and fixpoint state).
- **Future:** the harness may invoke `scripts/run/verify-bootstrap.sh` directly
  as the canonical Stage 0→1→2 gate once bootstrap is stable in CI; the
  contract above is written so that wiring is a one-line delegation.

No change to `verify-harness.sh` is required for issue #154; the integration
point is documented here for future CI work.
