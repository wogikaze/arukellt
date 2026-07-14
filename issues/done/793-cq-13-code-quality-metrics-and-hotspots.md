---
Status: done
Created: 2026-07-14
Updated: 2026-07-14
ID: 793
Track: tooling-contract
Depends on: "787, 792"
Orchestration class: completed
Orchestration upstream: None
Blocks v{N}: none
Priority: 1
Source: CQ closed-loop plan CQ-13
---

# 793 — CQ-13: code-quality metrics and hotspots

## Summary

Collect deterministic Ark metrics and rank review hotspots as advisory signals,
not as an absolute code-quality score.

## Acceptance

- [x] `quality report` and `quality report --json` render one deterministic model
- [x] `quality report --output PATH` writes the JSON model without changing the baseline
- [x] File/function size, parameters, nesting, complexity, imports, public surface, wrappers, long lines, TODOs, suppressions, churn, and centrality are collected
- [x] String and comment contents are excluded from brace and branch counts
- [x] Churn is `unknown` without usable Git history and does not fail collection
- [x] Hotspots show score inputs and reason signals and are documented as priorities, not scores
- [x] `docs/data/ark-code-quality-baseline.toml` stores count/p50/p75/p90/p95/max while retaining legacy counts
- [x] Baseline writes are explicit and increases require an issue and reason
- [x] `quality full` runs the report; `quality quick` does not collect churn
- [x] Collector, determinism, fallback, and baseline roundtrip tests pass

## Completion evidence

### Reopened by closure regression audit (2026-07-14)

`python3 scripts/check/check-ark-code-quality.py` raises `ValueError` while
parsing string metrics metadata as an inventory integer. Inventory and metrics
writers also do not share a section-preserving TOML model. The prior completion
evidence is retained below as history, but CQ-13 remains open until regression
tests and all canonical commands pass again.

### Closure regression repair (2026-07-14)

- Reproduced the false-done regression: the inventory check treated the string
  `metrics_baseline_reason` as an integer and raised `ValueError`.
- Replaced both writers' hand-maintained parsing/serialization with the shared
  `scripts/quality/baseline.py` model using Python `tomllib`.
- Split the baseline into `[inventory]`, `[counts]`, `[metrics_metadata]`, and
  `[metrics.*]`. Inventory writes preserve metrics metadata/distributions;
  metrics writes preserve inventory metadata/counts. Both writers were run
  consecutively against the repository baseline and the subsequent check and
  report passed.
- Malformed TOML, invalid types, and missing required keys now produce a
  concise failure. Enforced Ark sources with zero selected inputs and missing
  formatter/linter executions return failure instead of an empty success.
- Added regression coverage for string metadata, section preservation in both
  directions, malformed/type/missing-key failures, explicit-only writes,
  lower-only inventory updates, no-history churn, zero selections, and missing
  tools.
- `python3 -m unittest discover -s scripts/tests -p 'test_*.py'`: 97 passed.
- `python3 scripts/check/check-ark-code-quality.py`: pass after both explicit
  writers; inventory remained 1,900 files / 10,201 functions, with legacy
  ratchets `lines_ge_200 = 437`, `thin_wrappers = 1733`, and
  `single_function_files = 530` preserved.
- `python3 scripts/manager.py quality report`: pass; churn available in this
  checkout, deterministic distributions and advisory hotspot wording retained.
- `python3 scripts/manager.py quality full`: pass; formatter 1,981/1,981,
  linter 1,981/1,981, structure 0 errors, and metrics report completed.
- `python3 scripts/manager.py docs check`,
  `python3 scripts/check/check-docs-consistency.py`, and
  `python3 scripts/check/check-issue-health.py`: pass.

- `scripts/quality/metrics.py` provides the sanitized scanner, deterministic distributions, git fallback, hotspot model, JSON output, and explicit baseline writer.
- `docs/data/ark-code-quality-baseline.toml` retains `lines_ge_200 = 437` and `thin_wrappers = 1733` while adding all CQ-13 distribution keys with issue #793 metadata.
- `scripts/tests/test_quality_metrics.py` covers determinism, comment/string exclusion, no-history fallback, baseline roundtrip/non-update, and fmt/lint exit contracts.
- `quality report`, `--json`, and `--output .build/code-quality/report.json`: pass; written JSON has schema version 1 and 50 hotspots (2026-07-14).
- Repository collection: 1,900 compiler files, 10,201 functions, churn available. The output explicitly says it is advisory and not a quality score.
- `python3 -m unittest discover -s scripts/tests -p 'test_*.py'`: 92 tests passed (2026-07-14).

## Primary artifacts

- `scripts/quality/metrics.py`
- `docs/data/ark-code-quality-baseline.toml`

## References

- `docs/adr/ADR-047-code-quality-tooling-and-gates.md`
- `docs/adr/ADR-048-design-heuristics-application-order.md`
