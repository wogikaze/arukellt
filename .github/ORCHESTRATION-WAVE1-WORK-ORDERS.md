# Wave 1 — acceptance slice work orders (orchestration)

Each block is copied verbatim into the subagent prompt. Do not pass full issue files; use `issues/open/<file>` only as reference.

---

## WO-1 — impl-compiler / #064

- **AGENT_NAME**: impl-compiler
- **ISSUE_ID**: 064
- **ISSUE_TRACK**: wasm-feature
- **ISSUE_KIND**: implementation-ready
- **SUBTASK**: Emit a **non-empty** `metadata.code.branch_hint` custom section for at least one real MIR path that already carries `BranchHint::Likely` / `BranchHint::Unlikely` (issue status note said the section was stubbed with 0 entries). If precise bytecode-offset mapping is too large for one slice, land minimal correct entries + a regression test that parses emitted Wasm and asserts section length > 0 and references a known control instruction. Defer `@likely`/`@unlikely` source syntax (criterion 3) unless trivial.
- **PRIMARY_PATHS**: `crates/ark-wasm/src/emit/t3_wasm_gc/**`, `crates/ark-mir/**` (only if MIR annotation plumbing is required)
- **ALLOWED_ADJACENT_PATHS**: `tests/fixtures/**` if a new fixture is the smallest proof; `crates/ark-wasm/tests/**`
- **REQUIRED_VERIFICATION**: `bash scripts/run/verify-harness.sh --quick`; if MIR/emit touched, also `bash scripts/run/verify-harness.sh --cargo`
- **DONE_WHEN**: Wasm output contains non-empty branch_hint section for the chosen path; tests pass; acceptance slice documented in completion report.
- **STOP_IF**: No `BranchHint` in MIR anywhere to hang wiring on; or wasmtime validation requires spec you cannot satisfy in one slice.
- **COMMIT_MESSAGE_HINT**: `feat(wasm): populate branch_hint custom section for T3`

---

## WO-2 — impl-compiler / #282

- **AGENT_NAME**: impl-compiler
- **ISSUE_ID**: 282
- **ISSUE_TRACK**: corehir
- **ISSUE_KIND**: implementation-ready
- **SUBTASK**: **Verify** CoreHIR lowering for `Operand::LoopExpr`: `while` / `loop` / `for` fixtures compile on CoreHIR path and `validate_backend_legal_module` passes. Issue acceptance is all `[x]` but audit claims LoopExpr still backend-illegal — reconcile with code. If broken, fix in `crates/ark-mir/src/lower/**` with smallest diff; add or tighten one fixture if missing.
- **PRIMARY_PATHS**: `crates/ark-mir/src/lower/**`, `crates/ark-mir/src/mir.rs`
- **ALLOWED_ADJACENT_PATHS**: `tests/fixtures/**`, `crates/ark-driver/**` only if needed to run CoreHIR path
- **REQUIRED_VERIFICATION**: `bash scripts/run/verify-harness.sh --quick`; `bash scripts/run/verify-harness.sh --fixtures` if fixture added/changed
- **DONE_WHEN**: Concrete proof (commands + result) that loop-bearing programs pass CoreHIR path validation; or a fix + proof.
- **STOP_IF**: Blocker is entirely in another crate not listed; escalate in report without widening scope.
- **COMMIT_MESSAGE_HINT**: `fix(mir): CoreHIR LoopExpr lowering or verification proof`

---

## WO-3 — impl-component-model / #032

- **AGENT_NAME**: impl-component-model
- **ISSUE_ID**: 032
- **ISSUE_TRACK**: component-model
- **ISSUE_KIND**: implementation-ready
- **SUBTASK**: Address **export validation still errors on resources** (issue reopen reason). Smallest change so component export path accepts WIT resources per Key Files; add or run one regression that exports a resource constructor + method **or** documents exact remaining gap with a failing test reduced to minimal repro.
- **PRIMARY_PATHS**: `crates/ark-wasm/src/component/wit.rs`, `crates/ark-wasm/src/component/canonical_abi.rs`, `crates/ark-wasm/src/emit/t3_wasm_gc.rs` (and adjacent `t3_wasm_gc/**` as needed)
- **ALLOWED_ADJACENT_PATHS**: `crates/ark-wasm/src/component/wit_parse.rs`, component tests under `crates/ark-wasm/`
- **REQUIRED_VERIFICATION**: `bash scripts/run/verify-harness.sh --quick`; targeted `cargo test -p ark-wasm` if faster for the slice
- **DONE_WHEN**: Export validation no longer spuriously rejects resources for the covered case **or** a committed minimal repro + report explaining the single next step.
- **STOP_IF**: Requires language/parser changes outside component crate; STOP_IF and list dependency.
- **COMMIT_MESSAGE_HINT**: `fix(component): resource export validation for WIT`

---

## WO-4 — impl-benchmark / #110

- **AGENT_NAME**: impl-benchmark
- **ISSUE_ID**: 110
- **ISSUE_TRACK**: benchmark
- **ISSUE_KIND**: implementation-ready
- **SUBTASK**: **Audit** issue acceptance vs repo: `tests/baselines/perf/`, `scripts/run/verify-harness.sh --perf-gate`, `scripts/update-baselines.sh`. Issue body claims closure 2026-04-14 — verify each criterion (1–4) still holds on current `master`. If anything is missing or broken, implement the **minimal** fix; if already satisfied, add a one-line doc note or harness comment pointing to the proof paths (still commit if you touch files).
- **PRIMARY_PATHS**: `scripts/run/verify-harness.sh`, `scripts/update-baselines.sh`, `scripts/check/**` (only if perf gate lives there), `tests/baselines/perf/**`
- **ALLOWED_ADJACENT_PATHS**: `.github/workflows/**` only if CI wiring is explicitly broken
- **REQUIRED_VERIFICATION**: `bash scripts/run/verify-harness.sh --quick`; `bash scripts/run/verify-harness.sh --perf-gate` (or documented equivalent from harness `--help`)
- **DONE_WHEN**: Each acceptance item has yes/no in report with command output; any gap fixed or explicitly remains with STOP_IF.
- **STOP_IF**: Perf gate semantics undefined (no baseline file format).
- **COMMIT_MESSAGE_HINT**: `chore(bench): perf gate / baselines audit for #110`

---

## WO-5 — impl-vscode-ide / #453

- **AGENT_NAME**: impl-vscode-ide
- **ISSUE_ID**: 453
- **ISSUE_TRACK**: vscode-ide
- **ISSUE_KIND**: implementation-ready
- **SUBTASK**: Remove `suite.skip` from **Go to Definition** and **Hover** E2E suites in `extension.test.js`; keep binary-availability guard in `suiteSetup` so tests self-skip when the LSP binary is absent. Run the VS Code extension test suite; fix flakiness only if needed (e.g. wait strategy) within the test file.
- **PRIMARY_PATHS**: `extensions/arukellt-all-in-one/src/test/extension.test.js`, `extensions/arukellt-all-in-one/src/test/fixtures/**`
- **ALLOWED_ADJACENT_PATHS**: `extensions/arukellt-all-in-one/package.json` only if test script invocation requires it
- **REQUIRED_VERIFICATION**: `bash scripts/run/verify-harness.sh --quick` if harness covers extension tests; otherwise `npm test` or the repo-documented extension test command from `extensions/arukellt-all-in-one/README.md` / package scripts
- **DONE_WHEN**: No `suite.skip` on those two suites; tests pass or cleanly self-skip when binary missing; completion lists commands run.
- **STOP_IF**: CI cannot run VS Code tests locally — report and partial completion without fake passes.
- **COMMIT_MESSAGE_HINT**: `test(vscode): enable definition and hover E2E suites`
