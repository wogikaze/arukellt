# Scripts Consolidation Epic: Python CLI Refactoring

> **Status:** Implementation-ready (Epic)
> **Track:** tooling
> **Type:** Epic
> **Blocks:** none
> **Acceptance:** 0 checked / 5 open

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
- Establish shared Python library (`scripts/lib/`)
- Create CLI skeleton (`scripts/manager.py`)
- Migrate verify domain only (verify-harness.sh + check-docs-*.py)
- Keep other domains (selfhost, docs, perf, gate) as-is for now

**CLI design (subcommand-based):**
```bash
python scripts/manager.py verify quick
python scripts/manager.py verify fixtures
python scripts/manager.py verify docs
python scripts/manager.py verify size
python scripts/manager.py verify wat
python scripts/manager.py verify component
```

**Exit conditions for Phase 1:**
- [ ] `scripts/lib/` created with subprocess helpers, file ops, test harness integration
- [ ] `scripts/manager.py verify` subcommands pass behavioral contract tests
- [ ] CI updated to use `python scripts/manager.py verify` where applicable
- [ ] Shell scripts for verify kept as thin wrappers (forward to manager.py)
- [ ] Documentation updated for verify commands only

**Follow-up issues (to be created after Phase 1):**
- Issue #533: Selfhost domain migration
- Issue #534: Docs domain migration
- Issue #535: Perf domain migration
- Issue #536: Gate domain migration
- Issue #537: Shell script removal (after deprecation)

## Technical Decisions

**Python version floor:** Python 3.10 (matches CI environment)

**Dependency policy:** Standard library only (no external deps like click)

**Windows support:** Out of scope for Phase 1; Linux/macOS only (matches current shell scripts)

**Logging policy:** Preserve current stdout/stderr separation; no centralized logging yet

**Dry-run:** Add `--dry-run` flag to verify commands in Phase 1

**Backward compatibility:** Keep shell scripts as thin wrappers during transition; forward exit codes and args

**Test strategy:** Behavioral contracts (exit codes, stdout/stderr on success/failure), not coverage metrics

## Files (Phase 1)

**Create:**
- `scripts/lib/__init__.py`
- `scripts/lib/subprocess.py` (helpers for cargo, wasmtime, etc.)
- `scripts/lib/files.py` (file operations)
- `scripts/lib/harness.py` (test harness integration)
- `scripts/manager.py` (CLI skeleton with verify subcommands only)
- `scripts/tests/test_manager.py` (behavioral contract tests for verify)

**Modify:**
- `.github/workflows/ci.yml` (update verify calls to use manager.py)
- `scripts/run/verify-harness.sh` (convert to thin wrapper: exec python scripts/manager.py verify "$@")

**Keep unchanged (Phase 1):**
- All selfhost scripts
- All docs scripts (except those called by verify)
- All perf scripts
- All gate scripts

## Acceptance Criteria (Phase 1)

- [ ] Shared library (`scripts/lib/`) created and tested
- [ ] `manager.py verify quick` passes behavioral contract test
- [ ] `manager.py verify fixtures` passes behavioral contract test
- [ ] `manager.py verify docs` passes behavioral contract test
- [ ] `manager.py verify size` passes behavioral contract test
- [ ] `manager.py verify wat` passes behavioral contract test
- [ ] `manager.py verify component` passes behavioral contract test
- [ ] CI workflows using verify updated to call manager.py
- [ ] verify-harness.sh converted to thin wrapper (forwarding to manager.py)
- [ ] Documentation updated for verify commands

**Behavioral contract test definition:**
- Exit code matches shell script on success/failure
- stdout content matches shell script on success (for commands with output)
- stderr content matches shell script on failure (error messages)
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
