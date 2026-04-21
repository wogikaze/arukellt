# Scripts Directory

This directory contains utility scripts for building, testing, verifying, and maintaining the Arukellt compiler project.

## Overview

- **35 scripts total**: 24 shell scripts (`.sh`) and 11 Python scripts (`.py`)
- **Organized by domain**: `check/`, `run/`, `gen/`, `util/`, and domain-specific libraries
- **CLI manager**: `scripts/manager.py` provides unified interface (Phase 1 complete: verify domain)

## Directory Structure

```
scripts/
├── check/          # Validation and linting scripts
├── run/            # Test execution and verification scripts
├── gen/            # Documentation and index generation scripts
├── util/           # Utility libraries and helpers
├── lib/            # Generic Python utilities (subprocess, file ops)
├── verify/         # Verify-specific logic (fixtures, harness)
├── selfhost/       # Selfhost checks
├── perf/           # Performance checks
├── docs_domain/    # Docs checks
├── gate_domain/    # Gate checks
├── tests/          # Script tests
├── manager.py      # Unified CLI manager (verify domain complete)
├── compare-benchmarks.sh
├── update-baselines.sh
└── update-target-status.sh
```

## Scripts by Category

### Check Scripts (`scripts/check/`)

Validation and linting scripts for code quality, documentation, and consistency.

| Script | Language | Purpose | Usage |
|--------|----------|---------|-------|
| `check-asset-naming.sh` | Shell | Enforce snake_case for test fixtures and benchmarks | `bash scripts/check/check-asset-naming.sh [root]` |
| `check-diagnostic-codes.sh` | Shell | Verify error codes match between implementation and docs | `bash scripts/check/check-diagnostic-codes.sh` |
| `check-doc-examples.py` | Python | Extract and check all ```ark code blocks in docs | `python3 scripts/check/check-doc-examples.py [docs-dir]` |
| `check-docs-consistency.py` | Python | Extended docs consistency checker (bootstrap, capability, component state) | `python3 scripts/check/check-docs-consistency.py` |
| `check-docs-freshness.py` | Python | Check project-state.toml fixture counts and freshness | `python3 scripts/check/check-docs-freshness.py` |
| `check-generated-files.sh` | Shell | Validate generated file boundaries and ownership banners | `bash scripts/check/check-generated-files.sh [root]` |
| `check-links.sh` | Shell | Broken link / missing file reference checker for docs and issues | `bash scripts/check/check-links.sh` |
| `check-panic-audit.sh` | Shell | Detect unwrap/panic/todo/unimplemented in production code | `bash scripts/check/check-panic-audit.sh` |
| `check-playground-size.sh` | Shell | Playground Wasm and JS bundle size gates | `bash scripts/check/check-playground-size.sh [--wasm <file>] [--bundle-dir <dir>]` |
| `check-stdlib-manifest.sh` | Shell | Verify stdlib manifest matches resolve/typecheck/prelude.ark | `bash scripts/check/check-stdlib-manifest.sh` |

### Run Scripts (`scripts/run/`)

Test execution and verification scripts.

| Script | Language | Purpose | Usage |
|--------|----------|---------|-------|
| `compare-outputs.sh` | Shell | Compare Rust and selfhost compiler outputs for a given phase | `scripts/run/compare-outputs.sh <phase> [fixture.ark]` |
| `smoke-test-binary.sh` | Shell | Minimal smoke tests for a release binary | `./scripts/run/smoke-test-binary.sh [path-to-arukellt]` |
| `test-opt-equivalence.sh` | Shell | Verify optimization passes preserve semantics | `bash scripts/run/test-opt-equivalence.sh [--quick] [--fixture X]` |
| `test-package-workspace.sh` | Shell | Package-workspace manifest validation tests | `bash scripts/test-package-workspace.sh` |
| `verify-bootstrap.sh` | Shell | Bootstrap fixpoint verification for self-hosting | `bash scripts/run/verify-bootstrap.sh [--no-build]` |
| `wat-roundtrip.sh` | Shell | WAT roundtrip verification (compile → wasm2wat → wat2wasm) | `bash scripts/run/wat-roundtrip.sh` |

### Gen Scripts (`scripts/gen/`)

Documentation and index generation scripts.

| Script | Language | Purpose | Usage |
|--------|----------|---------|-------|
| `gen-harness-report.sh` | Shell | Parse cargo test harness output for CI artifact upload | `bash scripts/gen/gen-harness-report.sh [--baseline FILE] [--text] [LOG_FILE]` |
| `generate-docs.py` | Python | Generate documentation from source | `python3 scripts/gen/generate-docs.py [--check]` |
| `generate-issue-index.py` | Python | Auto-generate issue index and dependency graph | `python3 scripts/gen/generate-issue-index.py` |

### Util Scripts (`scripts/util/`)

Utility libraries and helpers.

| Script | Language | Purpose | Usage |
|--------|----------|---------|-------|
| `benchmark_runner.py` | Python | Benchmark runner for performance data collection | Used by perf-gate.sh and update-baselines.sh |
| `collect-baseline.py` | Python | Collect performance baseline data | `python3 scripts/util/collect-baseline.py` |

### Root Level Scripts

Scripts at the root of `scripts/` directory.

| Script | Language | Purpose | Usage |
|--------|----------|---------|-------|
| `compare-benchmarks.sh` | Shell | Cross-language benchmark comparison (C/Rust/Go vs Ark wasm) | `bash scripts/compare-benchmarks.sh [--quick] [--full]` |
| `update-baselines.sh` | Shell | Update performance baselines (compile time, runtime, binary size) | `bash scripts/update-baselines.sh [--dry-run]` |
| `update-target-status.sh` | Shell | Update docs/target-contract.md from CI test results | `scripts/update-target-status.sh [--dry-run] [INPUT_FILE]` |

### CLI Manager (`scripts/manager.py`)

Unified CLI interface for script consolidation.

**Phase 1 Complete (verify domain):**

```bash
python scripts/manager.py verify quick
python scripts/manager.py verify fixtures
python scripts/manager.py verify size
python scripts/manager.py verify wat
python scripts/manager.py verify component
```

**Status:** Phase 1 complete (verify domain migrated). Other domains (selfhost, docs, perf, gate) pending.

### Domain-Specific Libraries

Python libraries for domain-specific logic:

- **`scripts/lib/`**: Generic utilities (subprocess helpers, file operations)
- **`scripts/verify/`**: Verify-specific logic (harness integration, fixture loading)
- **`scripts/selfhost/`**: Selfhost checks
- **`scripts/perf/`**: Performance checks
- **`scripts/docs_domain/`**: Docs checks
- **`scripts/gate_domain/`**: Gate checks

## Common Environment Variables

Many scripts honor these environment variables:

- `ARUKELLT_BIN` - Path to arukellt binary (overrides default target/release/arukellt)
- `ARUKELLT_TARGET` - Target triple for compilation (e.g., wasm32-wasi-p1)
- `PLAYGROUND_WASM_LIMIT` - Playground Wasm size limit in bytes (default: 307200)
- `PLAYGROUND_BUNDLE_LIMIT` - Playground JS bundle limit in bytes (default: 524288)

## CI Integration

Scripts are called from CI workflows in `.github/workflows/`:

- `ci.yml` - Main CI pipeline (verify, tests, packaging, selfhost, etc.)
- `playground-ci.yml` - Playground CI (bundle size gates, Lighthouse audit)
- `pages.yml` - Documentation deployment

## See Also

- `docs/current-state.md` - Current user-visible behavior contract
- `docs/process/agent-harness.md` - Process and harness documentation
- `issues/open/531-scripts-consolidation-to-python-manager.md` - Scripts consolidation epic
