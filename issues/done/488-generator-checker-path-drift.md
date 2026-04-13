# Generator and checker path drift

**Status**: done
**Created**: 2026-04-13
**Updated**: 2026-04-13
**ID**: 488
**Track**: hygiene, docs
**Blocks v1 exit**: no
**Priority**: 70

## Created by audit — 2026-04-13

**Source**: `scripts/check/check-docs-consistency.py` and various docs reference generator scripts at old paths (`scripts/generate-docs.py`, `scripts/generate-issue-index.sh`) while actual files live under `scripts/gen/`. No open issue tracked this drift.

## Summary

Several scripts and docs still reference generator paths before the `scripts/gen/` reorganisation. This causes check scripts to silently pass or fail to invoke the intended tool.

Known drifted references:

- `scripts/check/check-docs-consistency.py` references `scripts/generate-issue-index.sh` → real path `scripts/gen/generate-issue-index.sh`
- docs/README.md and AGENTS.md reference `scripts/gen/generate-docs.py` (correct), but in-code comments or other docs may reference the old flat path

## Acceptance

- [x] All cross-references to generator scripts resolve to actual files
- [x] `scripts/check/check-docs-consistency.py` invokes generators at correct paths
- [x] `grep -rn 'scripts/generate-' .` returns zero hits outside `scripts/gen/` (no stale flat-path refs)

## Primary paths

- `scripts/check/check-docs-consistency.py`
- `scripts/gen/`
- `docs/`

## Required verification

- `bash scripts/run/verify-harness.sh --quick` passes
- `grep -rn 'scripts/generate-' . --include='*.py' --include='*.sh' --include='*.md' | grep -v 'scripts/gen/'` returns empty

## Close gate

- All generator references point to `scripts/gen/` paths
- check-docs-consistency runs end-to-end without path errors
