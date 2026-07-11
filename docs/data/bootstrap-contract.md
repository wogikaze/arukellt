# Bootstrap contract (structured)

> **Generated** from `docs/data/bootstrap-contract.toml` (ADR-029).

- Trust base: `pinned_wasm` → `bootstrap/arukellt-selfhost.wasm`
- Rust Stage 0: `False`
- Entrypoint: `scripts/run/arukellt-selfhost.sh`
- ADR: `docs/adr/ADR-029-selfhost-native-verification-contract.md`

- Wasm resolution order:
  1. `$ARUKELLT_SELFHOST_WASM`
  2. `.build/selfhost/arukellt-s3.wasm`
  3. `.build/selfhost/arukellt-s2-runtime.wasm`
  4. `.build/selfhost/arukellt-s2.wasm`
  5. `.bootstrap-build/arukellt-s2.wasm`
  6. `.build/selfhost/arukellt-pinned-bootstrap.wasm`
  7. `bootstrap/arukellt-selfhost.wasm`

## Stages

| ID | Name | Description | Artifact | Comparison |
|----|------|-------------|----------|------------|
| `0` | `trust_base` | Pinned selfhost wasm is the trust base (not a Rust compiler) | `bootstrap/arukellt-selfhost.wasm` | `n/a` |
| `build_s2` | `current_selfhost` | Pinned compiles src/compiler/main.ark → s2 | `.build/selfhost/arukellt-s2.wasm` | `build succeeds` |
| `fixpoint` | `s2_equals_s3` | sha256(s2) == sha256(s3) | `.build/selfhost/arukellt-s3.wasm` | `sha256` |

## Gates

| ID | Command | CI job |
|----|---------|--------|
| `fixpoint` | `python3 scripts/manager.py selfhost fixpoint` | `selfhost` |
| `fixture_parity` | `python3 scripts/manager.py selfhost fixture-parity` | `selfhost` |
| `cli_parity` | `python3 scripts/manager.py selfhost parity --mode --cli` | `selfhost` |
| `diag_parity` | `python3 scripts/manager.py selfhost diag-parity` | `selfhost` |

## Retired

| ID | Path | Reason | Archive |
|----|------|--------|---------|
| `verify-bootstrap-rust-stage0` | `scripts/run/verify-bootstrap.sh` | Rust Stage 0 cannot parse current selfhost source surface | `docs/history/reports/bootstrap-rust-era-verification.md` |
| `ARUKELLT_USE_RUST` | `env:ARUKELLT_USE_RUST` | Hard-fails in arukellt-selfhost.sh (#583 / ADR-029) | `docs/history/reports/bootstrap-rust-era-compiler-guide.md` |
