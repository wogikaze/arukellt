# Bootstrap context — LSP / capability / CLI issue batch

Reference for all workers and verifiers in this run. Read this first, then the
linked issue file. Do **not** re-discover the repo layout from scratch.

## Goal of this run

Advance these tracked issues (all `Status: open`, `Orchestration class:
implementation-ready`):

| ID | Track | Summary | Acceptance count |
|----|-------|---------|------------------|
| #334 | lsp-navigation | stdlib definition resolution (manifest-driven) | 4 |
| #335 | lsp-navigation | cross-file goto-definition | 4 |
| #338 | lsp-semantic | semantic references / documentHighlight | 4 |
| #450 | vscode-ide | goto-definition identifier-span | 5 |
| #451 | vscode-ide | hover semantic-only | 7 |
| #452 | vscode-ide | diagnostics false-positives (E0100 parity) | 5 |
| #236 | main | CLI / LSP stdio startup contract | 4 |
| #462 | extension | extension settings rationalization | 7 |
| #295 | capability | host API runtime tests | 6 |

## CRITICAL: the issue bodies cite a deleted code layout

Every one of these issue files was written against the old Rust workspace under
`crates/ark-*` (e.g. `crates/ark-lsp/src/server.rs`, `crates/ark-parser/src/ast.rs`,
`crates/ark-lsp/tests/lsp_e2e.rs`). **That layout no longer exists.** There is no
`crates/` directory in this repo. The implementation is the **selfhost compiler**
written in Arukellt under `src/compiler/`.

The authoritative pointers are the **`## Reopened by audit — 2026-06-12`** blocks
near the top of each issue file (the `Evidence files:` lists), not the stale
`crates/...` prose in the body. Translate every `crates/ark-*` reference to its
selfhost equivalent below.

## Selfhost code map (where the real work is)

LSP server (Arukellt source):

- `src/compiler/lsp/` — the LSP server modules:
  - `feature_definition.ark` — goto-definition / goto-type-definition (#335, #450)
  - `feature_hover.ark` — hover (#334 hover, #451)
  - `feature_completion.ark`, `completion.ark` — completion (#334)
  - `feature_symbol.ark`, `feature_workspace_symbol.ark` — symbol features (#450 spans)
  - `dispatch.ark`, `dispatch_features.ark`, `features.ark` — request routing (#338 references handler)
  - `diagnostics.ark`, `diag_*.ark` — diagnostics emit + JSON (#452)
  - `symbol_index*.ark` — project-wide symbol index from #333 (already landed); basis for cross-file (#335) and semantic references (#338)
  - `dispatch_lifecycle.ark`, `init_workspace.ark`, `state_*.ark` — initialize / initializationOptions / config (#462 server-side, #236 lifecycle)
- `src/compiler/analysis/` — `symbols.ark`, `symbol_types.ark`, `diagnostic_record.ark` — semantic symbol model used by navigation/references
- `src/compiler/parser/` + `src/compiler/parser.ark` — AST + parser; `let` / fn / param spans (#450 `name_span`)
- `src/compiler/resolver/` + `resolver.ark` — name resolution, prelude binding (#452 E0100 root cause)
- `src/compiler/typechecker/` — type inference (hover type info)
- `src/compiler/main/` — CLI entry/dispatch:
  - `editor.ark` — `cmd_lsp` / editor commands (#236 stdio contract)
  - `dispatch.ark`, `dispatch_ide.ark` — command dispatch (#236)
  - `args_*.ark`, `usage.ark`, `output.ark` — `--version` format, stdout/stderr separation (#236)
- `src/compiler/wasm/` — wasm emitter intrinsic handlers, incl. `call_host_io.ark` — host intrinsics (#295 clock/random)
- `std/manifest.toml` — canonical stdlib definitions (263 fns: signature/module/doc) — source of truth for #334 manifest-driven resolution

Extension (VS Code):

- `extensions/arukellt-all-in-one/package.json` — settings declarations (#462)
- `extensions/arukellt-all-in-one/src/extension.js` — client init / initializationOptions / actionable errors (#236, #462)
- `extensions/arukellt-all-in-one/src/test/extension.test.js` — E0100 E2E currently `test.skip` (#452)
- `extensions/arukellt-all-in-one/README.md` — settings table (#462)

Fixtures / tests:

- `tests/fixtures/selfhost/*.lsp-expected` — LSP golden fixtures (e.g. `lsp_hover_definition.lsp-expected`) for #450/#451/#452 regression
- `tests/fixtures/stdlib_io/`, `tests/fixtures/stdlib_env/`, `tests/fixtures/stdlib_time/` — host API fixtures (#295)
- `tests/fixtures/manifest.txt` — fixture registry (`run:` / `t3-run:` entries) (#295)

## #295 root cause (read before scoping clock/random)

The audit reopen for #295 says the selfhost emitter has **no handlers** for
`__intrinsic_clock_now`, `__intrinsic_clock_now_ms`, or `__intrinsic_random_i32`
(same root cause as reopened #051). `rg '__intrinsic_clock|__intrinsic_random'
src/compiler/wasm/` returns no dispatch handlers. So #295 is **not** a
fixtures-only task: the clock/random fixtures cannot lower on the selfhost path
until those intrinsic handlers are implemented in the wasm emitter
(`src/compiler/wasm/call_host_io.ark` and related dispatch). env/fs/process
fixtures may already be valid; clock/random need the intrinsic work first.
Close evidence must be **emitter-backed execution**, not fixture file presence.

## #236 note (avoid duplicating #634)

Wave 1 issue #634 (`selfhost LSP/DAP stdio transport entrypoint`) already wired
stdio JSON-RPC for `arukellt lsp`. Check `git log` / `cmd_lsp` current state before
re-implementing. #236 is the broader *contract* issue: confirm (a) `arukellt lsp`
stdio usage is documented + usable without a script-file workaround, (b)
`arukellt --version` returns a machine-readable (semver) format, (c) stdout =
protocol only / stderr = diagnostics+logs, (d) extension emits an actionable error
when the CLI is missing. Much may already hold; verify and document, fix the gaps.

## Skill routing

| Track | Skill |
|-------|-------|
| Compiler / LSP / parser / resolver | `.agents/impl-selfhost/SKILL.md` |
| Stdlib / host intrinsics | `.agents/impl-stdlib/SKILL.md` |
| CLI / scripts | `.agents/impl-cli/SKILL.md` |
| VS Code extension | `.agents/impl-vscode-ide/SKILL.md` |
| Scoped acceptance slices | `.agents/acceptance-slice-implementer/SKILL.md` |
| Close-review / hygiene | `.agents/reviewer/SKILL.md`, `.agents/verify/SKILL.md` |

## Completion contract (per AGENTS.md)

1. `python3 scripts/manager.py verify quick` exits 0 (full: `--full`).
2. Regenerate generated artifacts touched by the work:
   - issue index after issue file edits / moves: `python3 scripts/gen/generate-issue-index.py`
   - docs after manual doc-source changes: `python3 scripts/gen/generate-docs.py`
   - docs drift check: `python3 scripts/check/check-docs-consistency.py`
3. Update the issue file's `## Acceptance` checkboxes to reflect real, verified state.
   Only move a file from `issues/open/` to `issues/done/` when **every** criterion is
   met and verified by repo evidence (these issues were reopened precisely because
   they were falsely marked done — do not repeat that).
4. Commit focused changes; do not `git push` extra branches or touch unrelated files.

## Default verification command

```bash
python3 scripts/manager.py verify quick
```

Issue-specific commands live in each issue's body / acceptance section.
