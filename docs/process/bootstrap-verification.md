# Bootstrap Verification

> **Current-first.** This page is a thin pointer to the ADR-029 verification
> contract. Do not treat Rust Stage 0 documents as current.

## Canonical commands

```bash
# Fixpoint (pinned → s2 → s3; sha256(s2)==sha256(s3))
python3 scripts/manager.py selfhost fixpoint --build

# Normal fixture gate: one current compiler, parallel compile+validate for all
# run fixtures, bounded cross-domain execution smoke.
python3 scripts/manager.py selfhost fixture-parity
# Equivalent explicit runner with worker control:
python3 scripts/run/selfhost-fixture-test.py --workers 4

# Other selfhost lanes
python3 scripts/manager.py selfhost parity --mode --cli
python3 scripts/manager.py selfhost diag-parity
```

The old per-fixture `pinned compile + current compile + package + run` design is
not a normal verification lane. Pinned-vs-current fixture comparison is only a
bootstrap artifact refresh audit:

```bash
python3 scripts/run/selfhost-fixture-test.py --reference
```

Use `--execute-all` only when full current-runtime execution coverage is needed.
It still compiles each fixture once; it does not restore pinned duplication.

## Normative sources

| Topic | Document |
|-------|----------|
| Trust base / stages | [`../adr/ADR-029-selfhost-native-verification-contract.md`](../adr/ADR-029-selfhost-native-verification-contract.md) |
| Pinned artifact refresh | [`../../bootstrap/PROVENANCE.md`](../../bootstrap/PROVENANCE.md) |
| Operator guide | [`../compiler/bootstrap.md`](../compiler/bootstrap.md) |
| Status table | [`../state/compiler.md`](../state/compiler.md) |
| Entrypoint | [`../../scripts/run/arukellt-selfhost.sh`](../../scripts/run/arukellt-selfhost.sh) |

## Historical material

Rust-era bootstrap walkthroughs are archived under
[`../history/reports/`](../history/reports/). They are not executable current
contracts and are not referenced by the selfhost entrypoint.
