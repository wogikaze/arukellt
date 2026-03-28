# jco JavaScript interop smoke test

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-03-28
**ID**: 036
**Depends on**: 033
**Track**: component-model
**Blocks v1 exit**: no

## Summary

Validate that Arukellt components can be called from JavaScript using
[jco](https://github.com/bytecodealliance/jco). Add a Node.js-based smoke test that
transpiles a `.component.wasm` via `jco transpile`, imports the result, and verifies
correct output. Register this as an optional verify-harness gate (enabled with
`ARUKELLT_TEST_JCO=1`).

## Context

v2's reach goal is interoperability beyond wasmtime. jco is the official JavaScript
toolchain for Wasm components (maintained by the Bytecode Alliance). Calling an Arukellt
component from JavaScript (Node.js or browser) is a v2 completion criterion documented
in `docs/process/roadmap-v2.md` sections 2 (到達目標 item 4), 6 (実装タスク item 6), and
8 (完了条件 item 3).

The existing #033 covers wasmtime-based interop. This issue specifically covers the jco
path, which requires:

1. `jco transpile` — converts `.component.wasm` to a JS module tree
2. `jco componentize` (optional) — not needed for Arukellt-produced components
3. A Node.js test runner that imports the transpiled module and asserts output

### Why a separate issue

jco requires Node.js ≥18 and `npm`. This dependency is heavier than the existing CI
toolchain (Rust + wasmtime). Separating the issue allows:
- Marking #033 (wasmtime) done independently of jco availability
- Making the jco gate opt-in (`ARUKELLT_TEST_JCO=1`) without blocking the main harness

## Acceptance Criteria

- [ ] `jco` is listed as an optional CI dependency in `README.md` (or `docs/contributing.md`)
      with installation instructions (`npm install -g @bytecodealliance/jco`).
- [ ] A test fixture exists at `tests/component-interop/jco/calculator/` containing:
      - `calculator.ark` — Arukellt source exporting `pub fn add(a: i32, b: i32) -> i32`
        and `pub fn greet(name: String) -> String`
      - `test.mjs` — Node.js ES module that:
        1. Runs `jco transpile calculator.component.wasm -o ./out`
        2. `import`s the transpiled module
        3. Asserts `add(3, 4) === 7`
        4. Asserts `greet("World") === "Hello, World!"`
        5. Exits with code 0 on success, 1 on failure
      - `run.sh` — shell script that compiles `calculator.ark` to `calculator.component.wasm`
        then runs `node test.mjs`
- [ ] `scripts/verify-harness.sh` gains an optional check:
      - Skipped silently when `ARUKELLT_TEST_JCO=1` is not set
      - When set: runs `tests/component-interop/jco/calculator/run.sh` and reports
        pass/fail as check 18 (jco-interop)
- [ ] `docs/process/roadmap-v2.md` watchpoint #5 is satisfied: the `--with-jco` / env var
      opt-in mechanism is documented in `docs/platform/wasm-features.md`.

## Key Files

- `tests/component-interop/jco/calculator/calculator.ark`
- `tests/component-interop/jco/calculator/test.mjs`
- `tests/component-interop/jco/calculator/run.sh`
- `scripts/verify-harness.sh` — optional check 18
- `docs/platform/wasm-features.md` — jco usage docs
- `README.md` / `docs/contributing.md` — Node.js dependency note

## Notes

- The jco gate is **opt-in**. `scripts/verify-harness.sh` without `ARUKELLT_TEST_JCO=1`
  must still exit 0 at 17/17 (the existing component gate from #035). The jco gate is
  check 18, separate from the core 17-point harness.
- String crossing the component boundary (`greet`) is included to exercise the canonical
  ABI string lift/lower implemented in #029. An `add(i32, i32) -> i32` test alone would
  not catch string regressions.
- Browser environment (via `jco transpile --no-nodejs-compat`) is out of scope for v2.
  This issue targets Node.js only.
- If `jco transpile` produces a JS module that doesn't import correctly due to component
  format issues, this is a bug in #030/#031 (component wrapping / export adapters), not
  in this issue. File a bug on the relevant issue.
- jco version pin: fix the jco version in `run.sh` (`npm exec --package=@bytecodealliance/jco@<version>`)
  to avoid silent breakage from jco updates.
