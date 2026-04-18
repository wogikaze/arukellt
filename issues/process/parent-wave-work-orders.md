# Parent wave — work orders (dispatch copy)

Generated during parent orchestration. Each block is one subagent prompt.

---

## WO-1 — impl-compiler / #283

- **AGENT_NAME**: impl-compiler
- **ISSUE_ID**: 283
- **ISSUE_TRACK**: corehir
- **ISSUE_KIND**: implementation-ready
- **SUBTASK**: Issue acceptance is all `[x]` but audit claims `Operand::TryExpr` still backend-illegal. **Verify** current CoreHIR path: `?` / try fixtures compile with `--mir-select corehir-debug` (or project default), and `validate_backend_legal_module` passes. If broken, fix in `crates/ark-mir/src/lower/**` minimally; add/strengthen unit test like #282.
- **PRIMARY_PATHS**: `crates/ark-mir/src/lower/**`, `crates/ark-mir/src/mir.rs`
- **ALLOWED_ADJACENT_PATHS**: `tests/fixtures/**`, `crates/ark-driver/**` if needed to invoke CoreHIR
- **REQUIRED_VERIFICATION**: `bash scripts/run/verify-harness.sh --quick`; `bash scripts/run/verify-harness.sh --fixtures` if fixtures touched
- **DONE_WHEN**: Commands + results prove try/Desugar path is backend-legal or fixed with regression.
- **STOP_IF**: Root cause is outside `ark-mir` lower — report only.
- **COMMIT_MESSAGE_HINT**: `fix(mir): CoreHIR TryExpr verification or lowering`

---

## WO-2 — impl-component-model / #028

- **AGENT_NAME**: impl-component-model
- **ISSUE_ID**: 028
- **ISSUE_TRACK**: component-model
- **ISSUE_KIND**: implementation-ready
- **SUBTASK**: **Single gap from audit**: implement WIT `flags { ... }` parsing **or** wire `WitType::Flags` + **E0090** diagnostic when a function signature uses flags (per issue acceptance text). Do **not** attempt full `--wit` resolver injection in this slice unless trivial; stay in `crates/ark-wasm/src/component/**`.
- **PRIMARY_PATHS**: `crates/ark-wasm/src/component/wit_parse.rs`, `crates/ark-wasm/src/component/wit.rs`, `crates/ark-wasm/src/component/mod.rs`
- **ALLOWED_ADJACENT_PATHS**: `crates/ark-diagnostics/**` if E0090 needs a stable code path
- **REQUIRED_VERIFICATION**: `bash scripts/run/verify-harness.sh --quick`; `cargo test -p ark-wasm` as needed
- **DONE_WHEN**: Flags parse + E0090 on use, **or** documented STOP_IF with minimal test proving gap if truly blocked.
- **STOP_IF**: Diagnostic numbering conflicts with existing E00xx policy.
- **COMMIT_MESSAGE_HINT**: `feat(component): WIT flags parse and E0090 for unsupported codegen`

---

## WO-3 — impl-selfhost / #499

- **AGENT_NAME**: impl-selfhost
- **ISSUE_ID**: 499
- **ISSUE_TRACK**: selfhost
- **ISSUE_KIND**: implementation-ready
- **SUBTASK**: First acceptance only: **Selfhost parser recognises `|params| body` closure syntax** (AST + parser tests). Do not complete typechecker/lowering in this slice unless already trivial once syntax exists.
- **PRIMARY_PATHS**: `src/compiler/**` (parser/ast only per issue)
- **ALLOWED_ADJACENT_PATHS**: `tests/fixtures/**` under selfhost if needed
- **REQUIRED_VERIFICATION**: `bash scripts/run/verify-harness.sh --quick`
- **DONE_WHEN**: Parser accepts closure syntax with tests; checklist item 1 verifiable yes/no.
- **STOP_IF**: Grammar conflicts with existing `|` tokens — report and smallest partial.
- **COMMIT_MESSAGE_HINT**: `feat(selfhost): parse closure |params| body syntax`

---

## WO-4 — impl-stdlib / #521

- **AGENT_NAME**: impl-stdlib
- **ISSUE_ID**: 521
- **ISSUE_TRACK**: stdlib
- **ISSUE_KIND**: implementation-ready
- **SUBTASK**: Progress note claims parse contract landed; remaining gate is **`bash scripts/run/verify-harness.sh --fixtures`**. Run it; if failures are **outside** JSON contract, fix only if one-line / manifest; otherwise add issue progress note + ensure JSON-related fixtures pass. Goal: either full `--fixtures` green or narrow issue **Required verification** with repo evidence in the completion report.
- **PRIMARY_PATHS**: `std/json/mod.ark`, `tests/fixtures/stdlib_json/**`, `docs/stdlib/modules/json.md`
- **ALLOWED_ADJACENT_PATHS**: `scripts/run/verify-harness.sh` only for diagnosis
- **REQUIRED_VERIFICATION**: `bash scripts/run/verify-harness.sh --fixtures` (and `--quick`)
- **DONE_WHEN**: `--fixtures` pass **or** explicit documented subset with failing categories listed and issue text updated in same commit.
- **STOP_IF**: Fixture failures need large unrelated product work — STOP_IF and list top failure.
- **COMMIT_MESSAGE_HINT**: `test(stdlib): close #521 fixtures gate or narrow verification`

---

## WO-5 — impl-playground / #382

- **AGENT_NAME**: impl-playground
- **ISSUE_ID**: 382
- **ISSUE_TRACK**: playground
- **ISSUE_KIND**: implementation-ready
- **SUBTASK**: **Docs slice**: Update `docs/target-contract.md` and `docs/current-state.md` (and `crates/ark-target/src/lib.rs` registry comment if needed) so T2 / `wasm32-freestanding` status matches **current** repo (`t2_freestanding` emitter, fixtures). Align with `issues/done/501` state. Do not reimplement emitter.
- **PRIMARY_PATHS**: `docs/target-contract.md`, `docs/current-state.md`
- **ALLOWED_ADJACENT_PATHS**: `crates/ark-target/src/lib.rs`, `issues/open/382-playground-t2-freestanding.md` for progress note
- **REQUIRED_VERIFICATION**: `python3 scripts/check/check-docs-consistency.py`
- **DONE_WHEN**: Docs accurately describe T2 vs remaining gaps; consistency script passes.
- **STOP_IF**: Docs contract requires product code change to be truthful — note and partial doc update only.
- **COMMIT_MESSAGE_HINT**: `docs(playground): sync T2 target status with implementation`

---

## Wave barrier — completion (read 2026-04-18)

| WO | Agent | Issue | Status | Commit |
|----|-------|-------|--------|--------|
| 1 | impl-compiler | 283 | completed | `bad5178` |
| 2 | impl-component-model | 028 | completed | `1eb041c` |
| 3 | impl-selfhost | 499 | completed | `ac37aa1` |
| 4 | impl-stdlib | 521 | completed (narrowed `--fixtures` note) | `5953bd5` |
| 5 | impl-playground | 382 | completed | `373f763` |

**Close candidates:** None for full `issues/done/` without per-issue acceptance review; partial slices landed.

**Next wave proposal:** 039 module infra resolver slice; 154 verification-infra; 112 benchmark; 200 editor-runtime DAP; 283/028 follow-ups if any checkbox remains — re-read `index-meta.json` after updating issue headers.
