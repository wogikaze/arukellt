# Runnable Examples Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make every file in `example/*.ar` executable through `lang run` and testable through `lang test`.

**Architecture:** Extend the current compiler/interpreter vertically instead of adding a parallel executor. The parser and typechecker gain only the surface needed by the examples, the interpreter gets the builtin runtime for console, fs, ranges, lists, pipes, lambdas, and iter/unfold, and the CLI gains snapshot-style example testing.

**Tech Stack:** Rust workspace, `lang-core`, `lang-ir`, `lang-interp`, `lang-cli`, integration tests with `cargo test`

---

### Task 1: Lock down example runner behavior with failing CLI tests

**Files:**
- Modify: `crates/lang-cli/tests/cli.rs`
- Create: `example/*.stdout`

- [ ] Add failing integration tests that run representative examples via `lang run` and assert exact stdout.
- [ ] Add failing integration tests that call `lang test example/<name>.ar` and assert snapshot success.
- [ ] Run the targeted CLI tests and confirm they fail for missing syntax/runtime support.

### Task 2: Extend surface syntax in `lang-core`

**Files:**
- Modify: `crates/lang-core/src/lexer.rs`
- Modify: `crates/lang-core/src/ast.rs`
- Modify: `crates/lang-core/src/parser.rs`
- Modify: `crates/lang-core/src/types.rs`
- Modify: `crates/lang-core/tests/pipeline.rs`

- [ ] Add failing parser tests for bare imports, `i64`, omitted return type, lists, tuples, indexing, `<`, `%`, `or`, pipe, range literals, method chains, function references, and lambdas.
- [ ] Implement the minimal lexer/token changes needed by those tests.
- [ ] Implement parser/AST/type parsing changes to accept the example surface.
- [ ] Re-run the parser tests and keep them green before moving on.

### Task 3: Typecheck the example surface

**Files:**
- Modify: `crates/lang-core/src/typecheck.rs`
- Modify: `crates/lang-core/src/types.rs`
- Modify: `crates/lang-core/tests/pipeline.rs`

- [ ] Add failing typecheck tests for `Unit`, `Result<ok, err>`, `Fn<arg, result>`, builtins `Ok`/`Err`, function values, and pipe lowering semantics.
- [ ] Implement the smallest type-system changes needed for those tests.
- [ ] Re-run the typecheck tests and confirm the example-shaped sources compile.

### Task 4: Execute the example runtime in `lang-interp`

**Files:**
- Modify: `crates/lang-interp/src/lib.rs`
- Modify: `crates/lang-interp/tests/eval.rs`

- [ ] Add failing interpreter tests for lambdas/closures, list pipelines, range materialization, `string`, `join`, `sum`, `console.println`, `fs.read_text`, `iter.unfold`, `take`, tuple indexing, and `|>`.
- [ ] Implement the builtin runtime and closure evaluation needed by those tests.
- [ ] Re-run interpreter tests and confirm example behavior is covered.

### Task 5: Wire CLI example execution and snapshot testing

**Files:**
- Modify: `crates/lang-cli/src/commands.rs`
- Modify: `crates/lang-cli/src/cli.rs`
- Modify: `crates/lang-cli/tests/cli.rs`

- [ ] Make `lang run` execute `main()` for the examples and emit captured console stdout without extra `result:` noise for unit-returning programs.
- [ ] Make `lang test` fall back to snapshot testing using adjacent `.stdout` files when a source file has no `test_*` functions.
- [ ] Re-run CLI tests and keep the behavior stable.

### Task 6: Update example fixtures and docs, then verify end-to-end

**Files:**
- Modify: `README.md`
- Modify: `example/*.ar`
- Create: `example/*.stdout`

- [ ] Ensure every example source matches the implemented surface and has an expected stdout fixture.
- [ ] Run `cargo test`.
- [ ] Run `cargo run -p lang-cli -- run example/hello_world.ar`.
- [ ] Run `cargo run -p lang-cli -- test example/hello_world.ar`.
- [ ] Run a loop over every `example/*.ar` for both `lang run` and `lang test`, and confirm all pass.
- [ ] Commit the runnable examples and CLI/runtime changes in one commit.
