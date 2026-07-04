---
Status: open
Created: 2026-07-07
Updated: 2026-07-07
ID: 719
Track: tooling
Depends on: "715"
Orchestration class: implementation
Orchestration upstream: None
Blocks v{N}: none
Priority: 1
Source: ADR-041 Phase 2 — test execution model
---

# 719 — `arukellt test` execution harness (ADR-041 Phase 2)

## Summary

ADR-041 Phase 1 landed `test mod` / `test "..." {}` syntax, parser,
resolver, typechecker, and `arukellt test` CLI discovery. However,
`arukellt test` currently only **type-checks** test bodies — it does
not **execute** them. This issue implements Phase 2: a `cargo nextest`-
style test runner that compiles `test` blocks into executable Wasm
functions, runs them, and reports pass/fail per test.

## Current state

### What works (Phase 1, #715)

- `test "name" { ... }` / `test <fn> "name" { ... }` / `test mod "name" { ... }`
  syntax parsed (`src/compiler/parser/decl_test.ark`)
- Test declarations resolved in file scope (`resolver/bindings_test.ark`)
- Test bodies type-checked (`typechecker/body.ark`)
- `arukellt test <file>` discovers and lists tests, then runs `--check-only`
  compilation (`src/compiler/main/project_run.ark` `cmd_test`)
- 180+ in-file tests adopted across `std/` and `src/compiler/` (#715)

### What does not work

- Test bodies are **not lowered to MIR** — `decl_source` only processes
  `NK_FN` and `NK_IMPL`, so `NK_TEST_DECL` / `NK_TEST_MOD` are skipped
  (ADR-041 D5: "MIR lowering では test 宣言をスキップ")
- No Wasm code is generated for test bodies
- No runtime execution of `assert_*` calls
- No pass/fail reporting based on actual assertion results
- 1194 `.expected` golden files still required for fixture-parity harness

## Design (ADR-041 D7, Approach A)

### Strategy: synthesize test functions into a single Wasm module

Each `test` block is compiled into an independent Wasm function within
the same module. A synthesized `__test_main` function calls each test
function in sequence. The runtime traps (from `panic` / failed `assert`)
are caught to determine pass/fail per test.

### Phases

#### Phase 2a: MIR synthesis for test declarations

1. **`decl_source` extension**: Add `NK_TEST_DECL` / `NK_TEST_MOD`
   handling to `mir_decl_source` so test declarations are visible to
   the lowering pipeline (currently returns `COREHIR_DECL_UNKNOWN`).

2. **Test function synthesis**: For each `NK_TEST_DECL`, synthesize a
   MIR function `__test_<index>` with no params and no return. The
   function body is the test block's statement list, lowered via the
   existing `lower_stmt` infrastructure.

3. **`test <fn> "name"` handling**: For function-bound tests, the
   synthesized function has access to the target function via normal
   module scope (already resolved in Phase 1).

4. **`__test_main` synthesis**: After all test functions are lowered,
   synthesize a `__test_main` function that:
   - Calls each `__test_<index>` in order
   - Catches traps via a try/catch mechanism (or relies on Wasm trap
     for failure detection — see Phase 2b)

5. **Entry point selection**: When `--test` mode is active, emit
   `__test_main` as the Wasm `_start` entry instead of the normal
   `main` function.

#### Phase 2b: Runtime execution and reporting

Two sub-approaches for trap handling:

**Option B1 (simpler, recommended for initial impl)**:
- Each test function is compiled into the same Wasm module
- `__test_main` calls each test in sequence
- If a test traps (panic → unreachable), the whole module traps
- The CLI runner detects: trap = FAIL, clean exit = PASS
- **Limitation**: first failure stops the run (no continue-after-fail)
- **Mitigation**: `--fail-fast` is default; `--no-fail-fast` compiles
  each test as a separate Wasm module and runs them independently

**Option B2 (full `nextest` experience)**:
- Each test is compiled as a separate Wasm module
- The CLI runner executes each module independently via wasmtime
- Trap in one test does not affect others
- Supports parallel execution (`-j N`)
- **Cost**: N compilations for N tests (slower)

**Decision**: Start with B1 for simplicity. Add B2 as a follow-up
if compilation speed becomes a bottleneck.

#### Phase 2c: CLI integration

Update `cmd_test` in `src/compiler/main/project_run.ark`:

```
arukellt test <file.ark>              # run all tests, fail-fast
arukellt test <file.ark> --no-fail-fast  # run all, report all failures
arukellt test <file.ark> --filter <pattern>  # run matching tests only
arukellt test <dir>/                  # discover all .ark files
```

Output format (cargo-nextest inspired):

```
testing std/core/hash.ark
  PASS   hash::i32_zero        (0.3ms)
  PASS   hash::i32_neg         (0.2ms)
  PASS   hash::str_empty       (0.1ms)
  FAIL   hash::str_abc         (0.4ms)
    panic: assertion failed: expected 97, got 98
  PASS   hash::combine         (0.2ms)
  PASS   hash::combine_comm    (0.2ms)

  5 passed, 1 failed, 6 total (std/core/hash.ark)
```

Exit code: 0 if all pass, 1 if any fail.

#### Phase 2d: `.expected` migration

Once `arukellt test` can execute `test mod` blocks:

1. **Empty `.expected` files (609)**: These are already assert-based.
   Migrate the corresponding `.ark` fixtures to use `test mod` blocks
   instead of `fn main() { assert... }`, then delete the `.expected`
   file and update `manifest.txt` from `run:` to `test:`.

2. **Non-empty `.expected` files (585)**: Convert stdout-based golden
   tests to `assert_eq_string` / `assert_eq_i32` assertions inside
   `test mod` blocks. For benchmarks (16 files), keep `.expected` as
   correctness check alongside `bench` runner.

3. **fixture-parity harness update**: `scripts/selfhost/checks.py`
   `run_fixture_parity` should handle `test:` entries by running
   `arukellt test` instead of comparing stdout to `.expected`.

## Implementation order

1. **Phase 2a** — MIR synthesis (compiler changes)
2. **Phase 2b** — Runtime execution (CLI + wasmtime integration)
3. **Phase 2c** — CLI flags (`--filter`, `--no-fail-fast`)
4. **Phase 2d** — `.expected` migration (incremental, per-directory)

## Files to modify

### Compiler
- `src/compiler/mir/lower/decl_source.ark` — add test decl visibility
- `src/compiler/mir/lower/entry_decls.ark` — lower test decls
- `src/compiler/mir/lower/entry_emit_top.ark` — synthesize test functions
- `src/compiler/main/project_run.ark` — `cmd_test` execution mode
- `src/compiler/main/args_parse.ark` / `args_record.ark` — `--filter`, `--no-fail-fast`
- `src/compiler/wasm/` — entry point selection for test mode

### Scripts
- `scripts/selfhost/checks.py` — fixture-parity `test:` kind support
- `tests/fixtures/manifest.txt` — `run:` → `test:` for migrated fixtures

### Fixtures (incremental)
- 609 empty `.expected` files → `test mod` migration
- 585 non-empty `.expected` files → assertion conversion

## Dependencies

- #715 (in-file test adoption) — Phase 1 complete
- ADR-041 — design document (Phase 2 section D7)

## Risks

- **MIR synthesis complexity**: synthesizing functions from test blocks
  requires careful handling of the lowering context (scope, captures,
  mono instances). The test body is already type-checked, so types are
  available.
- **Trap handling in wasmtime**: wasmtime's `--invoke` API returns a
  trap on panic. The runner needs to distinguish "test trap" (assertion
  failure) from "compiler bug trap" (unexpected).
- **Compilation speed**: Option B1 compiles once (fast), B2 compiles N
  times (slow but isolated). Start with B1.
