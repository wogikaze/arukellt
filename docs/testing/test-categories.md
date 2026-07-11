# Test Category Classification Scheme

This document defines the classification scheme for all tests in the Arukellt project. Each test should belong to exactly one primary category, with clear responsibility, scope, and acceptance criteria.

## Failure report fields

Verification runners should attach these fields when a check fails:

- `category`: one category from this document, such as `fixture`,
  `component-interop`, `package-workspace`, `bootstrap`, or `editor-tooling`.
- `command`: the command that owns the failing check.
- `primary path`: the first repo path to inspect for the failure.

This metadata is required for package-workspace, fixture, component,
bootstrap, LSP, and extension checks so local logs identify the responsible
area without requiring path inference.

CI publishes the same category vocabulary in the `CI category summary` job. The
job writes the table to the GitHub run summary and uploads a
`ci-category-summary-<run_id>` artifact; each row includes the category state
and the responsible CI job/log pointer.

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

**CI Job**: `verification` (no separate unit-tests job in ci.yml)

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
- Fixture passes on supported targets (`wasm32`, `wasm32-gc`)
- Fixture manifest (`tests/fixtures/manifest.txt`) defines metadata
- Negative fixtures verify error cases

**Naming Convention**:
- Fixture files: `tests/fixtures/<category>/<name>.ark`
- Expected output: `tests/fixtures/<category>/<name>.expected`
- Diagnostics: `tests/fixtures/<category>/<name>.diag`

**CI Job**: `verification` (`manager.py verify` / verify modes)

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

**CI Job**: `verification`

---

### target-contract

**Responsibility**: Verify ABI / capability contracts for each canonical target (ADR-007).

**Scope**:
- `wasm32` (supported): linear-memory compatibility / AtCoder path
- `wasm32-gc` (primary): Wasm GC + default WASI P2 host profile; component emit
- `native-cpp` / `native-llvm` (scaffold): experimental; no wide guarantee
- `wasm32-gc` + WASI P3 host profile: not a separate language target; not started

Retired public names (must not appear as current contracts):
- `wasm32-freestanding` — ADR-007 hard error
- T1–T5 labels — historical only

**Acceptance Criteria**:
- Each shipped target has a contract test suite
- Tests verify target-specific ABI / host requirements
- Tests validate runtime behavior on the target
- Cross-target compatibility is verified where applicable

**Naming Convention**:
- Public docs / CLI: `wasm32`, `wasm32-gc`, `native-cpp`, `native-llvm`
- Fixture category prefixes such as `t3-run:` / `t3-compile:` are **historical internal names**
  for the primary (`wasm32-gc`) path; they are not public target IDs

**CI Job**: `verification` (target-contract checks inside harness)

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

**CI Job**: `verification` when `--component` / component gates run

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
- Scripts execute with correct environment, argument passthrough, and failure
  exit-code reporting

**Naming Convention**:
- Test files: `tests/package-workspace/<scenario>.rs` or `<scenario>.test.ark`
- Manifest files: `tests/package-workspace/<scenario>/ark.toml`
- Shell integration tests: `scripts/run/test-package-workspace.sh`

**CI Job**: local script lane — **not** a top-level ci.yml job

---

### bootstrap

**Responsibility**: Verify self-hosted compiler bootstrap (ADR-029).

**Scope**:
- Pinned wasm trust base (`bootstrap/arukellt-selfhost.wasm`)
- `python3 scripts/manager.py selfhost fixpoint` (`sha256(s2)==sha256(s3)`)
- Fixture / CLI / diagnostic parity lanes

**Acceptance Criteria**:
- `selfhost` CI job is green
- Fixpoint and parity commands in [`../compiler/bootstrap.md`](../compiler/bootstrap.md) pass
- No Rust Stage 0 / `ARUKELLT_USE_RUST` path

**Naming Convention**:
- Bootstrap sources: `src/compiler/**/*.ark`
- Pinned artifact: `bootstrap/arukellt-selfhost.wasm`

**CI Job**: `selfhost`

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

**CI Job**: `extension-tests` (+ LSP/DAP checks in verification)

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

**CI Job**: local / targeted — **not** a top-level ci.yml job

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

**CI Job**: harness / checklist — **not** a top-level ci.yml job

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
