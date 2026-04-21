# Scripts Consolidation Epic: Python CLI Refactoring

> **Status:** Phase 1 complete (Epic in progress)
> **Track:** tooling
> **Type:** Epic
> **Blocks:** none
> **Acceptance:** 9 checked / 0 open

## Why this must exist

The `scripts/` directory currently contains 35 scripts (29 shell + 6 Python) with significant redundancy and inconsistency:

**Current state:**
- 29 shell scripts (`.sh`) and 6 Python scripts (`.py`)
- Many shell scripts are thin wrappers around Python calls
- Selfhost checks split across 4 separate scripts
- Gate scripts duplicated across 3 files
- Inconsistent interfaces (some use `bash scripts/xxx.sh`, others `python3 scripts/yyy.py`)
- Maintenance burden: changes spread across multiple files
- Shell script portability concerns (bash version differences)

**Strategic decision:**
This is an **epic**, not a single implementation issue. We will:
1. First establish common Python library infrastructure
2. Migrate one domain at a time (verify → selfhost → docs → perf → gate)
3. Keep shell scripts as thin entry points during transition
4. Remove shell scripts only after deprecation completes

## First Phase: Verify Migration (Issue #532)

**Scope (Phase 1 only):**
- Establish shared Python library (`scripts/lib/`) for truly generic utilities
- Create domain-specific library (`scripts/verify/`) for verify-specific logic
- Create CLI skeleton (`scripts/manager.py`)
- Migrate verify domain only (verify-harness.sh core logic)
- **Exclude docs from Phase 1:** docs check scripts remain as-is; `verify docs` command will be removed in Issue #534 when docs domain is migrated

**CLI design (subcommand-based):**

```bash
python scripts/manager.py verify quick
python scripts/manager.py verify fixtures
python scripts/manager.py verify size
python scripts/manager.py verify wat
python scripts/manager.py verify component
```

**Note:** `verify docs` is **not** included in Phase 1. Current `verify-harness.sh --docs` calls check-docs-*.py directly; this behavior remains unchanged in Phase 1. Docs will become a separate top-level domain in Issue #534.

**Exit conditions for Phase 1:**
- [x] `scripts/lib/` created with truly generic utilities (subprocess helpers, file ops)
- [x] `scripts/verify/` created with verify-specific logic (harness integration, fixture loading)
- [x] `scripts/manager.py verify` subcommands pass behavioral contract tests
- [x] CI updated to use `python scripts/manager.py verify` where applicable
- [x] Shell scripts for verify kept as thin wrappers (forward to manager.py)
- [x] Documentation updated for verify commands only

**Follow-up issues (to be created after Phase 1):**
- Issue #533: Selfhost domain migration
- Issue #534: Docs domain migration
- Issue #535: Perf domain migration
- Issue #536: Gate domain migration
- Issue #537: Shell script removal (after deprecation)

## Technical Decisions

**Python version floor:** Python 3.10 (matches CI environment)

**Dependency policy:** Standard library only (no external deps like click)

**Windows support:** Out of scope for this epic. Current shell scripts are Linux/macOS only; Python CLI will maintain the same platform scope. Future Windows support would require separate issue with platform-specific subprocess handling.

**Logging policy:** Preserve current stdout/stderr separation; no centralized logging yet

**Dry-run:** Add `--dry-run` flag to verify commands in Phase 1

**Backward compatibility:** Keep shell scripts as thin wrappers during transition; forward exit codes and args

**Test strategy:** Behavioral contracts (exit codes, stdout/stderr on success/failure), not coverage metrics

## Files (Phase 1)

**Create:**
- `scripts/lib/__init__.py`
- `scripts/lib/subprocess.py` (truly generic subprocess helpers)
- `scripts/lib/files.py` (truly generic file operations)
- `scripts/verify/__init__.py`
- `scripts/verify/harness.py` (verify-specific harness integration)
- `scripts/verify/fixtures.py` (verify-specific fixture loading)
- `scripts/manager.py` (CLI skeleton with verify subcommands only)
- `scripts/tests/test_manager.py` (behavioral contract tests for verify)

**Modify:**
- `.github/workflows/ci.yml` (update verify calls to use manager.py with dual-run period)
- `scripts/manager.py` (convert to thin wrapper)

**Wrapper contract (verify-harness.sh):**

```bash
#!/bin/bash
# Thin wrapper: forward all args to manager.py
# Exit code: forward manager.py exit code exactly
# Signals: forward SIGINT/SIGTERM to manager.py (allow Python cleanup)
# Environment: inherit all env vars; pass ARUKELLT_BIN, ARUKELLT_TARGET to manager.py
# Args: forward "$@" exactly as-is
exec python3 scripts/manager.py verify "$@"
```

**CI migration strategy (dual-run period):**
- Week 1: CI runs both shell script and manager.py in parallel; both must pass
- Week 2: CI runs manager.py only; shell script kept as fallback
- Week 3: Remove shell script calls from CI; keep shell file for local use
- After Phase 1 completion: Shell script becomes optional local convenience

**Keep unchanged (Phase 1):**
- All selfhost scripts
- All docs scripts (except those called by verify)
- All perf scripts
- All gate scripts

## Acceptance Criteria (Phase 1)

- [x] Shared library (`scripts/lib/`) created and tested
- [x] Verify domain library (`scripts/verify/`) created and tested
- [x] `manager.py verify quick` passes behavioral contract test
- [x] `manager.py verify fixtures` passes behavioral contract test
- [x] `manager.py verify size` passes behavioral contract test
- [x] `manager.py verify wat` passes behavioral contract test
- [x] `manager.py verify component` passes behavioral contract test
- [x] CI workflows using verify updated to call manager.py (dual-run period)
- [x] verify-harness.sh converted to thin wrapper (forwarding to manager.py)
- [x] Documentation updated for verify commands

**Behavioral contract test definition:**
- Exit code matches shell script on success/failure (exact match required)
- stdout contains key success indicators (regex/contains, not exact match - allows for minor whitespace/timing differences)
- stderr contains key error messages (regex/contains, not exact match - allows for subprocess interleaving differences)
- Environment variables honored (ARUKELLT_BIN, ARUKELLT_TARGET, etc.)
- Working directory behavior preserved
- Signal handling (Ctrl+C) exits cleanly

## Dependencies

- None (standalone tooling refactoring)

## Blocks

- None

## Follow-up Issues

After Phase 1 completes, create:
- Issue #533: Selfhost domain migration (selfhost fixpoint, parity fixture, parity diag)
- Issue #534: Docs domain migration (docs check, docs regenerate)
- Issue #535: Perf domain migration (perf baseline, perf gate)
- Issue #536: Gate domain migration (gate local, gate pre-commit, gate pre-push)
- Issue #537: Shell script removal (after all domains migrated + deprecation period)

## Orchestration Notes

- Test-driven development: write behavioral contract test first, then implement
- Each subcommand independently testable
- Maintain backward compatibility via shell wrappers during transition
- Use standard library only (argparse, subprocess, pathlib, tempfile, shutil)
- Phase 1 is intentionally scoped to verify only; do not expand scope
