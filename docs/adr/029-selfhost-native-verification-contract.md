# ADR-029 — Selfhost-native verification contract

**Status**: Accepted
**Date**: 2026-04-22
**Issue**: [#585](../../issues/done/585-selfhost-native-verification-contract.md)
**Unblocks**: #583, #560, #561, #562, #563, #564 (Phase 5 Rust retirement)
**Related**: ADR-024 (selfhost MIR), ADR-027 (v3 selfhost completion)

## Context

The four canonical selfhost gates implemented in
`scripts/selfhost/checks.py` historically used the legacy Rust compiler
binary (`target/debug/arukellt`) as the **trusted base** for every
parity comparison:

| Gate | Pre-585 baseline | Comparison |
|------|------------------|------------|
| `selfhost fixpoint` | Rust binary compiles `src/compiler/main.ark` → `s1.wasm` | `sha256(s2) == sha256(s3)` (s2/s3 produced by selfhost) |
| `selfhost fixture-parity` | Rust binary compiles each fixture | `wasmtime`-execution of Rust output vs selfhost output (string equality) |
| `selfhost diag-parity` | Rust binary `check fixture.ark` produces the canonical diagnostic | Selfhost `check` output must contain the same `.diag` pattern |
| `selfhost parity --cli` | Rust binary `--version`/`--help`/exit codes | Byte-equal selfhost `--version`/`--help`/non-zero exit codes |

This contract blocks the Phase 5 Rust retirement chain (#583, #560–#564):
the gates hard-fail if `_find_arukellt()` cannot locate
`target/debug/arukellt`. As long as the Rust binary is the trusted base,
the Rust crates cannot be deleted.

The selfhost compiler reached fixpoint at #559: it bootstraps itself from
its own wasm output (`s2 == s3`), and `scripts/run/arukellt-selfhost.sh`
already runs the selfhost wasm as the user-facing default. The trusted
base for verification can be moved off the Rust binary without losing
behavioural coverage, provided we record a **byte-pinned** selfhost
artifact whose provenance is reproducible.

## Decision

Replace the legacy Rust-baseline contract with a **selfhost-native
verification contract** anchored on a single committed pinned-reference
wasm.

### Trusted base: `bootstrap/arukellt-selfhost.wasm`

A single wasm file, committed at `bootstrap/arukellt-selfhost.wasm`, is
the trusted base for every selfhost gate. It is exempted from the
repo-wide `*.wasm` `.gitignore` via an explicit allow-list. Provenance,
sha256, size, and reproducibility recipe live in
`bootstrap/PROVENANCE.md`. Refresh is explicit (`chore(bootstrap):
refresh pinned selfhost wasm`), never automatic, and must enumerate every
behavioural drift it introduces.

The artifact is currently 524 KiB — well under a 10 MiB ceiling we adopt
as a soft size budget. Future refreshes that approach the budget must be
called out in the refresh commit message.

### Reframed gate semantics

Every gate executes the selfhost compiler under `wasmtime`. None of them
read or shell out to `target/{debug,release}/arukellt`. None of them
require `cargo build`.

#### 1. `selfhost fixpoint` — bootstrap-from-pinned + Stage-3 fixpoint

```text
pinned (bootstrap/arukellt-selfhost.wasm)  ──▶  s2.wasm
s2.wasm  ──▶  s3.wasm
require: sha256(s2) == sha256(s3)
```

What is preserved from the pre-585 contract:
- The classical bootstrap fixpoint definition is unchanged
  (`sha256(s2) == sha256(s3)`).
- A drift in the selfhost compiler that breaks reproducibility
  (e.g. nondeterministic codegen) still fails the gate.

What changes:
- **Stage 0 is the pinned wasm, not the Rust compiler.** The Rust
  binary is no longer consulted.
- **Stage-1 byte-equality with the previous baseline is no longer
  asserted** as part of `fixpoint` — historically `s1` (Rust) and `s2`
  (selfhost) used different encodings anyway, so the only meaningful
  byte-pin was `s2 == s3`.

#### 2. `selfhost fixture-parity` — pinned-vs-current execution parity

```text
for each fixture in tests/fixtures/manifest.txt (run:):
    out_pinned   = wasmtime(pinned, "compile", fixture)
    out_current  = wasmtime(current_selfhost, "compile", fixture)
    run both wasms; require execution stdout/stderr/exit equal
```

`current_selfhost` is the Stage-2 wasm produced by `run_fixpoint`
(or rebuilt on demand from the pinned wasm + `src/compiler/main.ark`).

What is preserved:
- Behavioural coverage across the full `run:` fixture corpus
  (≥ 350 fixtures); execution-output equality is the same comparison
  the pre-585 contract used.
- The `FIXTURE_PARITY_SKIP` allow-list is preserved verbatim — known
  selfhost-only emitter shortcomings are still tracked, not silently
  dropped.
- `pass_count >= 10` floor is preserved (issue file requirement).

What changes:
- The baseline is the pinned selfhost, not the Rust compiler. The gate
  now detects **behavioural drift between the pinned baseline and the
  current source tree** instead of selfhost-vs-Rust drift.
- When `src/compiler/**` is unchanged, `current_selfhost == pinned`
  byte-for-byte and the gate passes vacuously (this is intentional —
  it is a regression detector, not a correctness oracle).
- When `src/compiler/**` drifts intentionally and the drift is
  behaviour-preserving, the gate still passes. When the drift is
  behaviour-changing, the gate fails until the maintainer either fixes
  the regression or refreshes the pinned wasm with a documented
  drift list (`bootstrap/PROVENANCE.md` refresh policy).

This is the strongest behaviour-preserving option of the two listed in
the issue file (selfhost-only-determinism is a strict subset of
pinned-vs-current parity, since the latter additionally detects
intentional-but-undocumented semantic drift).

#### 3. `selfhost diag-parity` — pure selfhost diagnostic snapshot

```text
for each fixture in tests/fixtures/manifest.txt (diag:):
    out = wasmtime(current_selfhost, "check", fixture)
    pattern = (fixture[:-4] + ".selfhost.diag")  if exists  else  (fixture[:-4] + ".diag")
    require: pattern in out
```

What is preserved:
- The committed `.diag` / `.selfhost.diag` golden files are the
  contract — exactly the same files that pre-585 selfhost used.
- The `DIAG_PARITY_SKIP` set is preserved verbatim.
- `pass_count >= 10` floor is preserved.

What changes:
- The gate no longer cross-checks the Rust binary's diagnostic output.
  Pre-585 fixtures that the Rust compiler classified "pattern not
  found; test may be stale" became `skip`; in the new contract any
  fixture whose `.diag` golden does not match the selfhost output is
  a `FAIL` (or must be added to `DIAG_PARITY_SKIP` with a tracking
  issue). This is a **strictly stronger** assertion in the long run,
  but at the moment of the migration the live `pass_count` is the
  same (≈ 11) because every previously passing fixture still passes.

#### 4. `selfhost parity --cli` — pure selfhost CLI snapshot

```text
require: wasmtime(current_selfhost, "--version") == tests/snapshots/selfhost/cli-version.txt
require: wasmtime(current_selfhost, "--help")    == tests/snapshots/selfhost/cli-help.txt
require: wasmtime(current_selfhost, "foobar_unknown_cmd").returncode != 0
require: for cmd in {compile, check, run}: wasmtime(current_selfhost, cmd).returncode != 0
```

What is preserved:
- The "non-zero exit on unknown / no-args" assertions are unchanged.
- The `--version` / `--help` text format is now a tracked snapshot
  rather than an inter-implementation comparison.

What changes:
- Goldens live under `tests/snapshots/selfhost/`. Intentional CLI text
  changes require updating the golden in the same commit.

### What is explicitly NOT in scope

- Deleting any Rust crate (`crates/**`) — that is #560–#564.
- Removing `ARUKELLT_USE_RUST=1` opt-in from
  `scripts/run/arukellt-selfhost.sh` — that is #583.
- Changing any selfhost source (`src/compiler/**`) or fixture
  (`tests/fixtures/**`).
- Changing `scripts/manager.py` CLI surface — `selfhost
  {fixpoint,fixture-parity,diag-parity,parity}` is preserved.

## Baseline gate counts at adoption (commit `662c3f58`)

| Gate | Result | Notes |
|------|--------|-------|
| `selfhost fixpoint` | PASS | s2 sha256 = `c16e32ef…0cc` (built from pinned `3a035037…f2c`) |
| `selfhost fixture-parity` | PASS | 321 PASS, 0 FAIL, 41 SKIP (16 selfhost wasm-trap, 23 selfhost-compile timeout under wasmtime, 2 explicit `FIXTURE_PARITY_SKIP`) |
| `selfhost diag-parity` | PASS | 12 PASS, 22 SKIP, 0 FAIL |
| `selfhost parity --cli` | PASS | 6 PASS, 0 FAIL |

These four lines are reproducible on a fresh clone with `cargo clean`
and `target/debug/arukellt` removed (verified at adoption time —
see #585 close note).

## Consequences

### Positive

- Phase 5 Rust retirement (#560–#564) is unblocked. `target/debug/arukellt`
  is no longer a verification dependency.
- Fresh clones can verify the selfhost compiler with only `wasmtime` and
  `python3` installed — no Rust toolchain, no `cargo build`.
- The trusted base is byte-pinned and reproducible from a git SHA.
- Drift between intentional source changes and the pinned baseline
  surfaces as gate failures, not silent regressions.

### Negative / accepted trade-offs

- Refreshing the pinned wasm becomes a maintenance ritual. Mitigation:
  refresh policy documented in `bootstrap/PROVENANCE.md`.
- The repository now carries a 524 KiB binary artifact in git. Mitigation:
  artifact is small, refresh frequency is low (only on intentional
  semantic changes), and the alternative (re-bootstrapping from a
  pre-built blob fetched at CI time) introduces a network/availability
  dependency that we judge worse.
- `fixture-parity` no longer cross-checks an independent implementation.
  Mitigation: the previous Rust-vs-selfhost comparison is being retired
  anyway by Phase 5; a stronger oracle (e.g. spec-derived fixtures with
  expected-output goldens) is tracked separately and out of scope here.

## Alternatives considered

1. **Selfhost-only determinism** (compile each fixture twice, require
   bit-identical wasm). Rejected: weaker than pinned-vs-current parity
   and would not detect intentional-but-undocumented semantic drift.
2. **Network-fetched pinned wasm** (download from a release asset
   instead of committing the binary). Rejected: introduces an
   availability dependency for verification on a fresh clone.
3. **Generate fixture-output goldens at commit time** and compare
   current selfhost runs against them. Rejected for this slice (would
   require a much larger fixture-output corpus and changes under
   `tests/fixtures/**` which is FORBIDDEN here); deferred to a
   follow-on issue.

## Verification

Per #585 acceptance:

```bash
rm -f target/debug/arukellt
python3 scripts/manager.py selfhost fixpoint        # PASS
python3 scripts/manager.py selfhost fixture-parity  # PASS
python3 scripts/manager.py selfhost diag-parity     # PASS
python3 scripts/manager.py selfhost parity --mode --cli  # PASS
```

The `cargo build --workspace --exclude ark-llvm` build still succeeds —
this slice does not delete any Rust crate.
