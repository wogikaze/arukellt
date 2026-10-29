# Bootstrap contract (structured)

> **Generated** from `docs/data/bootstrap-contract.toml` (ADR-029).

- Trust base: `pinned_wasm` → `bootstrap/arukellt-selfhost.wasm`
- Entrypoint: `scripts/run/arukellt-selfhost.sh`
- ADR: `docs/adr/ADR-029-selfhost-native-verification-contract.md`

- Wasm resolution order:
  1. `$ARUKELLT_SELFHOST_WASM`
  2. `bootstrap/arukellt-selfhost.wasm`
  3. `.build/selfhost/arukellt-s2-runtime.wasm`
  4. `.build/selfhost/arukellt-s3.wasm`
  5. `.build/selfhost/arukellt-s2.wasm`
  6. `.bootstrap-build/arukellt-s2.wasm`
  7. `.build/selfhost/arukellt-pinned-bootstrap.wasm`

## Stages

| ID | Name | Description | Artifact | Command | Comparison |
|----|------|-------------|----------|---------|------------|
| `0` | `trust_base` | Pinned wasm32-gc/wasi-p2 component is the trust base; it emits the same wasm32/wasi-p1 memory32 s2 as the previous core pin (56e7ff6a) in ~40 s and reproduces its own gc core (93aa0e6b) | `bootstrap/arukellt-selfhost.wasm` | — | `n/a` |
| `build_s2` | `current_selfhost` | Pinned gc component compiles src/compiler/main.ark → s2 under the 8192-page memory32 contract in ~40 s (stage-2 only, AOT-cached; the mode-stripped overlay runs with --bootstrap-allow-shared-param-mutation) | `.build/selfhost/arukellt-s2.wasm` | `python3 scripts/manager.py selfhost build-compiler` | `build succeeds` |
| `fixpoint` | `s2_equals_s3` | sha256(s2) == sha256(s3); the gc component builds both stages in ~40 s each (ADR-029 gate; not for routine s2 refresh) | `.build/selfhost/arukellt-s3.wasm` | `python3 scripts/manager.py selfhost fixpoint` | `sha256` |

## Gates

| ID | Command | CI job |
|----|---------|--------|
| `fixpoint` | `python3 scripts/manager.py selfhost fixpoint` | `selfhost` |
| `fixture_test` | `python3 scripts/manager.py selfhost fixture-parity` | `verification` |
| `cli_parity` | `python3 scripts/manager.py selfhost parity --mode --cli` | `selfhost` |
| `diag_parity` | `python3 scripts/manager.py selfhost diag-parity` | `selfhost` |

## Retired

| ID | Path | Reason | Archive |
|----|------|--------|---------|
| `ARUKELLT_USE_RUST` | `env:ARUKELLT_USE_RUST` | Hard-fails in arukellt-selfhost.sh (#583 / ADR-029) | `docs/history/reports/bootstrap-rust-era-compiler-guide.md` |
