# AGENTS.md

This repository is the **Arukellt** language toolchain: compiler/runtime backends, standard library, CLI, tests, and documentation.

## Repository Boundary

This repo contains:

- compiler and runtime implementation under `crates/`
- standard library sources under `std/`
- CLI integration via selfhost wrapper (`scripts/run/arukellt-selfhost.sh`)
- fixture / benchmark / verification infrastructure under `tests/`, `benchmarks/`, and `scripts/`
- user-facing and design documentation under `docs/`

## Primary Source of Truth

Use these in order, depending on the question:

- **Current user-visible behavior**: `docs/current-state.md`
- **Current open work queue**: `issues/open/index.md`
- **Current dependency ordering**: `issues/open/dependency-graph.md`
- **Completed tracked work**: `issues/done/`
- **Design decisions / rationale**: `docs/adr/`
- **Verification contract**: `scripts/manager.py` (`python scripts/manager.py verify`)
- **Generated docs contract**: `scripts/gen/generate-docs.py`

## Current Work Surface

The active open queue is the generated issue index under `issues/open/`.
At the time of writing, the queue is centered on:

- WASI Preview 2 native component output
- `std::host::*` namespace rollout
- shared host capability facades across T1 / T3

Do not infer active work from old roadmap prose when `issues/open/index.md` disagrees.

## Documentation Rules

- Treat `docs/current-state.md` as the current behavior contract.
- Many landing pages are generated. After changing manual doc sources, regenerate docs with:

```bash
python3 scripts/gen/generate-docs.py
```

- Check for docs drift with:

```bash
python3 scripts/check/check-docs-consistency.py
```

- If queue structure changes, regenerate issue indexes with:

```bash
python3 scripts/gen/generate-issue-index.py
```

## Completion Criteria

Work is complete when the relevant scope is updated and verification passes.
For tracked issue work, that normally means:

1. `python scripts/manager.py verify` exits with status 0
2. generated artifacts touched by the work are regenerated and included
3. relevant docs / ADRs are updated when behavior changed
4. tracked issue files move from `issues/open/` to `issues/done/` when the task itself is completed
5. commits stay focused to the files changed for that task
6. **all changes for the work unit are committed** before ending the turn (do not ask the user whether to commit)

## Commit Policy

Agents **must commit before ending a turn** when work is complete. Do not ask the user
whether to commit. This policy **overrides** global Cursor User Rules that say
"only commit when requested" (see `.cursor/rules/git-commits.mdc`, `alwaysApply: true`).

- **Every completed work unit must be committed** in the same session, after verify (or the relevant gate) passes.
- A turn that leaves uncommitted implementation, docs, indexes, or regenerated artifacts is **incomplete**.
- Do not include unrelated changes; do not commit secrets.
- Do not `git push` unless the user explicitly asks.
- If pre-commit fails, fix and create a **new** commit (do not amend unless hooks auto-modified files on your commit).
- Orchestration-only commits (issue moves, audit reports, indexes) stay separate from product implementation when both land in one session.
- Use HEREDOC for commit messages (`git commit -m "$(cat <<'EOF' ... EOF)"`).

Project Rules（Cursor Settings → Rules に表示される `git-commits` / `AGENTS`）で自律コミットは有効。
グローバル User Rules の有無は `.cursor/REPLACE-USER-RULE-commit.md` を参照（多くの環境では追加作業不要）。

## Verification Loop

- Quick pass: `python scripts/manager.py verify quick`
- Full pass: `python scripts/manager.py verify --full`

### T3 WASM Validation Cache & Parallelism

The T3 fixture WASM validation gate (`scripts/check/check-t3-wasm-validate.py`)
supports content-hash-based caching and parallel compilation:

- **Cache**: Results are cached in `.build/t3-cache/` keyed by SHA-256 of
  the compiler wasm + fixture source.  Repeat runs complete in <1s when
  nothing has changed.  Use `--no-cache` to bypass.
- **Parallel**: Uncached fixtures are compiled in parallel using
  `ProcessPoolExecutor`.  Use `-j N` to set the number of workers
  (default: half of CPU cores).  Example: `python3 scripts/check/check-t3-wasm-validate.py -j 6`

## Tooling Notes

- Prefer `ig` for code search.
- Generated docs and manifest-backed stdlib reference pages should be regenerated, not hand-maintained.

## API Design Principles

The stdlib is migrating from function-centric free functions to
**trait-first, type-first method syntax** (ADR-036, issue #709).

- **Prefer method syntax** `s.split(sep)`, `v.push(x)`, `n.to_string()`
  over free-function syntax `split(s, sep)`, `push(v, x)`, `i32_to_string(n)`.
- **Prefer `impl Type` blocks** for new public APIs. Define methods on
  the type they operate on, not as standalone functions in a module.
- **Prefer associated functions** `Vec::new()`, `String::from("x")` over
  monomorphic constructors `Vec_new_i32()`, `String_from("x")`.
- Free functions (`split(s, sep)`) remain valid for internal helpers,
  compiler/runtime bridges, and prelude thin wrappers, but should not
  be the primary user-facing API surface.
- Monomorphic helpers (`*_i32`, `*_i64`, `*_f64`) are implementation
  details or temporary bridges — not user-facing API.

References: ADR-036 (trait-stdlib-redesign), ADR-004-P4 (method-syntax
evaluation), issue #709 (trait-first API policy), issue #703
(monomorphic API bold cutover).

## Agent Skills

- Repo-local agent skill sources live under `.agents/`.
- Each skill is organized as a directory containing `SKILL.md` (English) and optionally `SKILL-ja.md` (Japanese).
- These include implementation specialists (impl-*), design agents (design-*), verification agents (verify-*), repo context (arukellt-repo-context), and acceptance slice implementation (acceptance-slice-implementer).
- Copy or symlink the `.agents/*/SKILL.md` files into your agent skills directory manually if needed.
- Autonomous multi-worktree orchestration prompts live under `.agents/prompts/`:
  - `autonomous-parent-orchestrator.md` — FSM-based parent orchestrator
  - `autonomous-child-worker.md` — FSM-based child implementation agent
  - `start-autonomous-loop.md` — launcher prompt
- Legacy orchestration prompts remain in `prompts/` (orchestration.md, exec-selfhost.md, subagent-*.md).

Current skills:
- Implementation specialists: impl-benchmark, impl-cli, impl-compiler, impl-component-model, impl-editor-runtime, impl-language-docs, impl-playground, impl-runtime, impl-selfhost, impl-selfhost-retirement, impl-stdlib, impl-vscode-ide
- Design agents: design-language, design-selfhost-mir, design-stdlib
- Verification agents: verify
- Other: acceptance-slice-implementer, arukellt-repo-context

## Markdown Navigation

- When reading large Markdown files such as `README.md`, docs, ADRs, or issue indexes, prefer `markdive` over loading the whole file at once.
- Use `npx markdive` so the workflow works even when the CLI is not globally installed.
- Recommended flow:

```bash
npx markdive dive <file> --depth 2
npx markdive dive <file> --path <section-id> --depth 2
npx markdive read <file> --path <section-id>
```

- First inspect structure with `dive`, then drill down with `--path`, and only then read the target section with `read`.
- Fall back to normal file reads only when `markdive` cannot handle the document.

## Selfhost Compiler Module Layout

The selfhost compiler sources live under `src/compiler/`. After modularization (May 2026), the file structure is:

### Entry Point & Driver

- `main.ark` — CLI entry point, argument parsing
- `driver.ark` — compilation pipeline orchestration

### Lex/Parse/Resolve

- `lexer.ark` (1,040 lines) — tokenizer
- `lexer_kinds.ark` (283 lines) — 69 TK_* token kind constants
- `parser.ark` (2,602 lines) — AST parsing (recursive descent + Pratt)
- `parser_kinds.ark` (385 lines) — NK_*/OP_*/UOP_* node kind constants
- `resolver.ark` (987 lines) — name resolution
- `resolver_kinds.ark` (162 lines) — SYM_*/NK_* symbol kind constants
- `hir.ark` — HIR type definitions

### Type Checker

- `typechecker.ark` (1,365 lines) — type inference, unification
- `typechecker_kinds.ark` (241 lines) — TY_*/NK_*/OP_* type kind constants

### MIR (Mid-level IR)

- `mir_opcodes.ark` (224 lines) — 54 MIR opcode constants
- `mir_ir.ark` (910 lines) — MIR type definitions (MirInst, MirBlock, MirFunction, MirModule) + SSA renaming infrastructure
- `mir_type_info.ark` (292 lines) — type system structures (MonoInstance, TypeInfo, etc.)
- `mir_lower.ark` (3,877 lines) — HIR→MIR lowering (LowerCtx + 41 lowering functions)
- `mir_dump.ark` (1,145 lines) — MIR dump/debug + entry point + instruction tag analysis

### Wasm Emitter

- `emitter.ark` (2,906 lines) — main Wasm binary emitter (was 13,374 before refactoring)
- `emit_opcodes.ark` (125 lines) — 105 Wasm opcode constants
- `emit_writer.ark` (156 lines) — LEB128/binary writer
- `emit_scratch.ark` (123 lines) — 28 scratch register constants
- `emit_inst_ctx.ark` (111 lines) — SelfEmitCtx context struct
- `emit_inst_const.ark` (57 lines) — 4 constant instruction handlers
- `emit_inst_locals.ark` (23 lines) — 2 local get/set handlers
- `emit_inst_arith.ark` (229 lines) — 22 arithmetic instruction handlers
- `emit_inst_convert.ark` (67 lines) — 9 type conversion handlers
- `emit_inst_control.ark` (47 lines) — 7 control flow handlers
- `emit_inst_struct.ark` (142 lines) — 6 struct/array instruction handlers
- `emit_intrinsic_string.ark` (3,788 lines) — 33 string intrinsic handlers
- `emit_intrinsic_math.ark` (3,160 lines) — 30 math/parse/sort intrinsic handlers
- `emit_intrinsic_vec.ark` (1,127 lines) — 15 vec intrinsic handlers
- `emit_intrinsic_io.ark` (1,667 lines) — 35 I/O/env/fs/assert/misc intrinsic handlers
- `emit_wat.ark` (190 lines) — WAT text format emitter

### Component Model

- `component_emitter.ark` — WASI Preview 2 component output

### Other

- `diagnostics.ark` — error/warning infrastructure
- `analysis.ark` — IDE analysis API
- `lsp.ark` — Language Server Protocol
