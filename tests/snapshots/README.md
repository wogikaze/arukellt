# Snapshot Tests

This directory contains committed output snapshots used to detect
unintended changes in the compiler's intermediate representations and
diagnostic messages.

## Directory Layout

```
tests/snapshots/
├── mir/           # MIR phase-dump snapshots
├── diagnostics/   # Diagnostic-message snapshots
└── README.md
```

## Purpose

| Directory      | What it captures | When it changes |
|----------------|------------------|-----------------|
| `mir/`         | MIR output produced by `ARUKELLT_DUMP_PHASES` for a fixed set of fixtures | Intentional IR redesign, optimisation pass changes |
| `diagnostics/` | Rendered error/warning messages for known-bad fixtures | Diagnostic wording or format changes |

Snapshots are **not** performance data — see `tests/baselines/` for
benchmark baselines and `docs/process/benchmark-plan.md` for the
performance-regression workflow.

## Updating Snapshots

```bash
# Regenerate all snapshots
bash scripts/update-snapshots.sh

# MIR only
bash scripts/update-snapshots.sh --mir

# Diagnostics only
bash scripts/update-snapshots.sh --diag
```

After updating, review the diff carefully:

```bash
git diff tests/snapshots/
```

If the changes are intentional, commit them together with the code change
that caused the difference.

## Adding a New Snapshot Fixture

1. Add the `.ark` file path to the `MIR_FIXTURES` array in
   `scripts/update-snapshots.sh` (for MIR snapshots), or ensure the
   fixture has a matching `.diag` file in `tests/fixtures/diagnostics/`
   (for diagnostic snapshots).
2. Run `bash scripts/update-snapshots.sh`.
3. Commit the new snapshot file.

## Relationship to Baselines

See `docs/process/snapshot-baseline-policy.md` for the full distinction
between snapshots and baselines.
