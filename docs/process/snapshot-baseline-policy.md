# Snapshot vs Baseline Policy

This document defines the responsibilities and update workflows for
**snapshots** and **baselines** in the Arukellt test infrastructure.

---

## Definitions

| Concept    | Location | Content | Governance |
|------------|----------|---------|------------|
| **Snapshot** | `tests/snapshots/` | Deterministic compiler output (MIR dumps, diagnostic messages) | Updated via `scripts/run/update-snapshots.sh`; committed alongside the code change that caused the diff |
| **Baseline** | `tests/baselines/` | Quantitative performance data (compile time, binary size, runtime) | Updated via `scripts/util/collect-baseline.py`; reviewed before commit to confirm regressions are intentional |

## When to Update

### Snapshots

Snapshots change when the compiler's textual output changes — for
example, after modifying MIR lowering, adding a new optimisation pass,
or rewording a diagnostic message.

**Workflow:**

1. Make the code change.
2. Run `bash scripts/run/update-snapshots.sh`.
3. Run `git diff tests/snapshots/` and confirm every difference is
   expected.
4. Commit the updated snapshot files in the same PR as the code change.

### Baselines

Baselines change when performance characteristics shift — for example,
after adding a new optimisation pass that reduces binary size or changes
compile time.

**Workflow:**

1. Make the code change.
2. Run `python3 scripts/util/collect-baseline.py` (or `mise bench:update-baseline`).
3. Compare results against the previous baseline using
   `mise bench:compare`.
4. Commit updated baselines only after confirming the regression (if any)
   is acceptable per the thresholds in `docs/process/benchmark-plan.md`.

## Verification Harness Integration

`scripts/manager.py` treats snapshots and baselines differently:

- **Baselines**: The `--perf-gate` flag enables quantitative regression
  checks (compile ≤ +20 %, runtime ≤ +10 %, binary size ≤ +15 %).
- **Snapshots**: Snapshot freshness is verified by the fixture harness
  and diagnostic tests.  A stale snapshot causes a test failure, not a
  harness-level gate, so developers can update snapshots without
  blocking the entire pipeline.

## Key Principles

1. **Snapshots are deterministic** — identical input must produce
   identical output.  Non-deterministic elements (HashMap iteration
   order, timestamps) must be excluded or normalised.
2. **Baselines are statistical** — small fluctuations are expected;
   only threshold-exceeding regressions are actionable.
3. **Both are committed** — neither snapshots nor baselines are
   `.gitignore`d.  They serve as the reviewable source of truth for
   "what the compiler does" and "how fast it does it".
4. **Update scripts are the single entry point** — never hand-edit
   snapshot or baseline files.
