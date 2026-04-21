# Scripts Consolidation to Single Python Manager

> **Status:** Implementation-ready
> **Track:** tooling
> **Blocks:** none
> **Acceptance:** 0 checked / 5 open

## Problem

The `scripts/` directory currently contains 35 scripts (29 shell + 6 Python) with significant redundancy and inconsistency:

**Current state:**
- 29 shell scripts (`.sh`) and 6 Python scripts (`.py`)
- Many shell scripts are thin wrappers around Python calls
- Selfhost checks split across 4 separate scripts: `check-selfhost-parity.sh`, `check-selfhost-fixpoint.sh`, `check-selfhost-fixture-parity.sh`, `check-selfhost-diagnostic-parity.sh`
- Gate scripts duplicated: `ci-full-local.sh`, `pre-commit-verify.sh`, `pre-push-verify.sh` all call `verify-harness.sh`
- 90% of actual logic is already in Python, yet shell scripts persist as entry points

**Issues:**
- Inconsistent interface (some use `bash scripts/xxx.sh`, others `python3 scripts/yyy.py`)
- Maintenance burden: changes spread across multiple files
- Shell script portability concerns (bash version differences)
- No single source of truth for CLI interface
- Difficult to test shell scripts in isolation

## Solution

Consolidate all scripts into a single Python CLI manager with test-driven development.

### Architecture

**New entry point:** `scripts/manager.py`

**CLI interface:**
```bash
python scripts/manager.py verify --quick
python scripts/manager.py verify --fixtures
python scripts/manager.py verify --docs
python scripts/manager.py verify --size --wat
python scripts/manager.py verify --component

python scripts/manager.py selfhost --fixpoint
python scripts/manager.py selfhost --parity --fixture
python scripts/manager.py selfhost --parity --diag

python scripts/manager.py docs --check
python scripts/manager.py docs --regenerate

python scripts/manager.py perf --baseline
python scripts/manager.py perf --gate

python scripts/manager.py gate --local
python scripts/manager.py gate --pre-commit
python scripts/manager.py gate --pre-push
```

**Internal structure:**
```
scripts/manager.py
├── CLI argument parsing (argparse or click)
├── Command handlers:
│   ├── verify: consolidate verify-harness.sh logic
│   ├── selfhost: consolidate selfhost check scripts
│   ├── docs: consolidate doc check/generation
│   ├── perf: consolidate perf gate/baseline
│   └── gate: consolidate gate scripts
└── Shared utilities:
    ├── subprocess helpers (for cargo, wasmtime, etc.)
    ├── file operations
    └── test harness integration
```

### Migration Strategy (Test-Driven)

**Phase 1: Test infrastructure**
- [ ] Create `scripts/tests/test_manager.py` with pytest
- [ ] Add tests for each existing script's behavior
- [ ] Capture current shell script exit codes and outputs as test expectations

**Phase 2: Implement verify command**
- [ ] Implement `manager.py verify --quick` (matches verify-harness.sh --quick)
- [ ] Add test: verify_quick_success
- [ ] Implement `manager.py verify --fixtures` (matches verify-harness.sh --fixtures)
- [ ] Add test: verify_fixtures_success
- [ ] Implement remaining verify flags (--docs, --size, --wat, --component)
- [ ] Add tests for each flag

**Phase 3: Implement selfhost command**
- [ ] Implement `manager.py selfhost --fixpoint` (matches check-selfhost-fixpoint.sh)
- [ ] Add test: selfhost_fixpoint_success
- [ ] Implement `manager.py selfhost --parity --fixture` (matches check-selfhost-fixture-parity.sh)
- [ ] Add test: selfhost_parity_fixture_success
- [ ] Implement `manager.py selfhost --parity --diag` (matches check-selfhost-diagnostic-parity.sh)
- [ ] Add test: selfhost_parity_diag_success

**Phase 4: Implement docs command**
- [ ] Implement `manager.py docs --check` (consolidates check-docs-*.py calls)
- [ ] Add test: docs_check_success
- [ ] Implement `manager.py docs --regenerate` (matches generate-docs.py)
- [ ] Add test: docs_regenerate_success

**Phase 5: Implement perf command**
- [ ] Implement `manager.py perf --baseline` (matches update-baselines.sh)
- [ ] Add test: perf_baseline_success
- [ ] Implement `manager.py perf --gate` (matches perf-gate.sh)
- [ ] Add test: perf_gate_success

**Phase 6: Implement gate command**
- [ ] Implement `manager.py gate --local` (matches ci-full-local.sh)
- [ ] Add test: gate_local_success
- [ ] Implement `manager.py gate --pre-commit` (matches pre-commit-verify.sh)
- [ ] Add test: gate_precommit_success
- [ ] Implement `manager.py gate --pre-push` (matches pre-push-verify.sh)
- [ ] Add test: gate_prepush_success

**Phase 7: CI migration**
- [ ] Update `.github/workflows/ci.yml` to use `python scripts/manager.py`
- [ ] Update `.github/workflows/playground-ci.yml` if affected
- [ ] Test CI with new manager.py
- [ ] Ensure all CI jobs pass

**Phase 8: Deprecate shell scripts**
- [ ] Add deprecation warnings to shell scripts pointing to manager.py
- [ ] Update documentation (README.md, docs/process/*)
- [ ] Update AGENTS.md with new command patterns
- [ ] Wait 1-2 weeks for adoption

**Phase 9: Remove shell scripts**
- [ ] Delete all deprecated shell scripts
- [ ] Update .gitignore if needed
- [ ] Final verification that all workflows use manager.py

## Files

**Create:**
- `scripts/manager.py` (new unified CLI)
- `scripts/tests/test_manager.py` (test suite)
- `scripts/tests/__init__.py` (test package)

**Modify:**
- `.github/workflows/ci.yml` (update script calls)
- `README.md` (update command examples)
- `docs/process/bootstrap-verification.md` (if references scripts)
- `AGENTS.md` (update command patterns)

**Delete (Phase 9):**
- All shell scripts in `scripts/check/` (except those requiring shell-specific ops)
- All shell scripts in `scripts/gate/`
- All shell scripts in `scripts/run/` (except verify-bootstrap.sh if needed)
- Root-level scripts: `compare-benchmarks.sh`, `update-baselines.sh`, `update-target-status.sh` (if not shell-dependent)

**Keep as shell (if truly needed):**
- `scripts/update-target-status.sh` (if requires git operations better suited to shell)
- Any scripts requiring specific shell built-ins not easily replicated in Python

## Acceptance Criteria

- [ ] All existing shell script behaviors have corresponding tests
- [ ] `manager.py` passes all tests
- [ ] CI workflows updated and passing with manager.py
- [ ] Documentation updated with new command patterns
- [ ] Shell scripts deprecated with warnings
- [ ] Shell scripts removed after deprecation period
- [ ] No functionality lost in migration
- [ ] Test coverage > 80% for manager.py

## Dependencies

- None (standalone tooling refactoring)

## Blocks

- None

## Orchestration Notes

- Test-driven development is mandatory: write test first, then implement
- Each command should be independently testable
- Maintain backward compatibility during migration phase
- Use existing Python utilities (argparse, subprocess, pathlib)
