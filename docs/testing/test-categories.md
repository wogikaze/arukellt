# Test Category Classification Scheme

This document defines the classification scheme for all tests in the Arukellt project. Each test should belong to exactly one primary category, with clear responsibility, scope, and acceptance criteria.

## Categories

### unit

**Responsibility**: Verify individual functions and modules in isolation.

**Scope**:
- Single function behavior
- Module-internal logic
- Data structure operations
- Utility functions

**Acceptance Criteria**:
- Tests are fast (< 100ms per test)
- No external dependencies (file I/O, network, subprocess)
- Deterministic results
- No side effects on shared state

**Naming Convention**:
- Test files: `tests/unit/<module>_test.rs` or `<module>.test.ark`
- Test functions: `test_<function_name>` or `test_<scenario>`

**CI Job**: `verification-unit`

---

### fixture

**Responsibility**: Verify language features and compiler correctness through end-to-end examples.

**Scope**:
- Language syntax and semantics
- Type system correctness
- Compiler pipeline behavior
- Standard library functions
- Error messages and diagnostics

**Acceptance Criteria**:
- Each fixture has a `.expected` file defining expected output
- Fixture passes on all supported targets (T1, T3)
- Fixture manifest (`tests/fixtures/manifest.txt`) defines metadata
- Negative fixtures verify error cases

**Naming Convention**:
- Fixture files: `tests/fixtures/<category>/<name>.ark`
- Expected output: `tests/fixtures/<category>/<name>.expected`
- Diagnostics: `tests/fixtures/<category>/<name>.diag`

**CI Job**: `verification-harness-quick` (subset), `verification-harness-full` (full)

---

### integration

**Responsibility**: Verify interaction between multiple modules or subsystems.

**Scope**:
- Cross-module compilation
- Multi-file projects
- Dependency resolution
- Workspace behavior
- Build pipeline integration

**Acceptance Criteria**:
- Tests involve multiple files or modules
- Tests verify end-to-end workflows
- Tests are deterministic and repeatable

**Naming Convention**:
- Test files: `tests/integration/<scenario>.rs` or `<scenario>.test.ark`
- Test directories: `tests/integration/<scenario>/` (for multi-file tests)

**CI Job**: `verification-integration`

---

### target-contract

**Responsibility**: Verify ABI contracts for each target tier (T1-T5).

**Scope**:
- T1 (wasm32-wasi-p1): Core Wasm compatibility
- T2 (wasm32-freestanding): Freestanding target
- T3 (wasm32-wasi-p2): GC-native Wasm
- T4 (native): LLVM native backend
- T5 (wasm32-wasi-p3): Future target

**Acceptance Criteria**:
- Each target has a contract test suite
- Tests verify target-specific ABI requirements
- Tests validate runtime behavior on the target
- Cross-target compatibility is verified where applicable

**Naming Convention**:
- Test files: `tests/target/<target>_<contract>.rs`
- Target names: `t1`, `t2`, `t3`, `t4`, `t5`

**CI Job**: `verification-target-contract`

---

### component-interop

**Responsibility**: Verify Component Model interoperability and WIT bindings.

**Scope**:
- `--emit component` output validity
- WIT generation and round-trip
- Component import/export
- Canonical ABI compliance
- Cross-component function calls

**Acceptance Criteria**:
- Component files pass `wasm-tools validate`
- WIT files can be parsed and round-tripped
- Exported functions are callable from other components
- Imported functions are correctly bound

**Naming Convention**:
- Test files: `tests/component-interop/<scenario>.rs` or `<scenario>.ark`
- Component outputs: `tests/component-interop/<scenario>.component.wasm`
- WIT outputs: `tests/component-interop/<scenario>.wit`

**CI Job**: `verification-component-interop`

---

### package-workspace

**Responsibility**: Verify package and workspace management functionality.

**Scope**:
- `ark.toml` parsing and validation
- Workspace resolution
- Dependency resolution
- Script execution
- Manifest resolution
- Package discovery

**Acceptance Criteria**:
- Valid `ark.toml` files are correctly parsed
- Invalid `ark.toml` files produce clear error messages
- Workspace dependencies are correctly resolved
- Scripts execute with correct environment

**Naming Convention**:
- Test files: `tests/package-workspace/<scenario>.rs` or `<scenario>.test.ark`
- Manifest files: `tests/package-workspace/<scenario>/ark.toml`

**CI Job**: `verification-package-workspace`

---

### bootstrap

**Responsibility**: Verify self-hosted compiler bootstrap process.

**Scope**:
- Stage 0 (Rust) → Stage 1 (Arukellt) compilation
- Stage 1 → Stage 2 fixpoint
- Bootstrap script correctness
- Bootstrap parity verification

**Acceptance Criteria**:
- `scripts/run/verify-bootstrap.sh` passes all stages
- Stage 2 produces identical output to Stage 1 (fixpoint)
- Bootstrap is deterministic
- Parity with Rust implementation is verified

**Naming Convention**:
- Test files: `tests/bootstrap/<stage>.rs`
- Bootstrap sources: `src/compiler/*.ark`

**CI Job**: `verification-bootstrap`

---

### editor-tooling

**Responsibility**: Verify editor integration (LSP, DAP, etc.).

**Scope**:
- LSP server functionality
- Language server protocol compliance
- Debug adapter functionality
- Editor extension behavior
- Code navigation and completion

**Acceptance Criteria**:
- LSP server starts and responds to requests
- Diagnostics are correctly reported
- Go-to-definition works correctly
- Hover provides accurate information
- Debug adapter can launch and debug programs

**Naming Convention**:
- Test files: `tests/editor-tooling/<feature>.test.ts` (for VS Code extension)
- LSP tests: `tests/editor-tooling/lsp_<feature>.rs`

**CI Job**: `verification-editor-tooling`

---

### perf

**Responsibility**: Verify performance characteristics and regression detection.

**Scope**:
- Compilation time
- Execution time
- Memory usage
- Binary size
- Benchmark comparisons

**Acceptance Criteria**:
- Performance baselines are defined in `tests/baselines/perf/`
- Performance regressions are detected in CI
- Benchmarks are reproducible
- Performance metrics are tracked over time

**Naming Convention**:
- Benchmark files: `benchmarks/<name>.ark`
- Baseline files: `tests/baselines/perf/<name>.json`

**CI Job**: `verification-perf` (separate from correctness gate)

---

### determinism

**Responsibility**: Verify deterministic compilation and execution.

**Scope**:
- Same input produces same output bytes
- No non-deterministic sources (timestamps, random seeds)
- Reproducible builds

**Acceptance Criteria**:
- Compiling the same source twice produces identical Wasm binaries
- Running the same program twice produces identical output
- No hidden sources of non-determinism

**Naming Convention**:
- Test files: `tests/determinism/<scenario>.rs`

**CI Job**: `verification-determinism`

---

## Category Assignment Guidelines

1. **Primary category**: Each test belongs to exactly one primary category
2. **Secondary concerns**: If a test touches multiple concerns, choose the primary responsibility
3. **File structure**: Use directory structure to reflect category where possible
4. **Naming**: Follow naming conventions for easy identification

## Migration Path

Existing tests should be migrated to this classification scheme incrementally:
1. Assign category to existing test files
2. Rename files to match naming conventions
3. Move files to appropriate directories
4. Update CI job assignments
