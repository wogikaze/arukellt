# Bootstrap Verification

> **Current-first.** This page is a thin pointer to the ADR-029 verification
> contract. Do not treat Rust Stage 0 documents as current.

## Canonical commands

```bash
# Fixpoint (pinned → s2 → s3; sha256(s2)==sha256(s3))
python3 scripts/manager.py selfhost fixpoint --build

# Parity lanes
python3 scripts/manager.py selfhost fixture-parity
python3 scripts/manager.py selfhost parity --mode --cli
python3 scripts/manager.py selfhost diag-parity
```

## Normative sources

| Topic | Document |
|-------|----------|
| Trust base / stages | [`../adr/ADR-029-selfhost-native-verification-contract.md`](../adr/ADR-029-selfhost-native-verification-contract.md) |
| Operator guide | [`../compiler/bootstrap.md`](../compiler/bootstrap.md) |
| Status table | [`../state/compiler.md`](../state/compiler.md) |
| Entrypoint | [`../../scripts/run/arukellt-selfhost.sh`](../../scripts/run/arukellt-selfhost.sh) |

## Retired

- `scripts/run/verify-bootstrap.sh` as the current attainment gate (Rust-era).
  Archive: [`../history/reports/bootstrap-rust-era-verification.md`](../history/reports/bootstrap-rust-era-verification.md).
