# Diagnostic Parity (current)

> Current verification is **selfhost diag-parity** under ADR-029 — not a Rust CLI comparison.
> The Rust-era comparison tables are archived at
> [`../history/reports/diagnostic-parity-rust-era.md`](../history/reports/diagnostic-parity-rust-era.md).

## Canonical command

```bash
python3 scripts/manager.py selfhost diag-parity
```

## Contract

Structured diagnostic field parity for the selfhost compiler against the
checked-in expectations / prior selfhost baselines used by the gate.
Error codes, severity, and structured fields must remain stable unless an ADR
documents a deliberate diagnostic change.

## See also

- [`bootstrap.md`](bootstrap.md)
- [`../adr/ADR-029-selfhost-native-verification-contract.md`](../adr/ADR-029-selfhost-native-verification-contract.md)
