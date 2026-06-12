# Bootstrap context — open issues implementation

Reference for all wave workers. Do not re-discover from scratch; read this first, then the linked issue file.

## Source of truth (read order)

1. `docs/current-state.md` — user-visible behavior contract
2. `issues/open/index.md` — dependency order + orchestration class
3. `issues/open/dependency-graph.md` — blocks/depends graph
4. `issues/open/index-meta.json` — machine-readable deps + acceptance counts
5. `scripts/manager.py verify quick` — completion gate (22 checks at time of planning)

## Queue hygiene

- **59** open issues are `implementation-ready` with satisfied open-issue dependencies (628 orchestration-done counts as satisfied for #634).
- **Blocked** (`blocked-by-upstream`, `partially-blocked`, `design-ready`): skip until upstream closes — see `issues/blocked/` and index `Orchestration` column.
- **Selfhost target**: implementation uses `src/compiler/*.ark` + `scripts/run/arukellt-selfhost.sh`, not deleted `crates/*` / `arukellt_host`.
- After issue moves or acceptance edits: `python3 scripts/gen/generate-issue-index.py`.
- After behavior/docs manifest changes: `python3 scripts/gen/generate-docs.py`.

## Priority lanes (audit 2026-06-12)

| Lane | Issues | Why first |
|------|--------|-----------|
| LSP transport | #634 | Blocks v1; VS Code stdio contract false today |
| Playground checker | #500 → #472 | Product claim false; close-gate test exists |
| WASI P2 gate | #510 → #074 | P2-native imports unblock component path |
| Host honesty | #633 | After #446/#447 or manifest/docs-only slice |
| Hygiene | #418 → #422 | Missing `check-orphan-inventory` script |

## Wave 1 scope (this plan)

| Task | Issue | Primary paths | Notes |
|------|-------|---------------|-------|
| implement-634-lsp-stdio | #634 | `src/compiler/main/editor.ark`, `dispatch.ark`, `lsp.ark`, `dap.ark` | `cmd_lsp` currently requires script file; extension uses stdio |
| implement-500-472-playground-typecheck | #500, #472 | `playground/src/**` | `typecheckSource()` wraps parse only; use selfhost check/typecheck path |
| implement-418-orphan-inventory | #418 | `scripts/check/`, `scripts/manager.py` | Script cited in false-done audit is absent |
| implement-137-host-gating | #137 | `src/compiler/resolver/`, `std/manifest.toml`, `tests/fixtures/target_gating/` | T3-only module gating missing in selfhost |

## Skill routing

| Track | Skill |
|-------|-------|
| Compiler / LSP / DAP | `.agents/impl-selfhost/SKILL.md` |
| Playground | `.agents/impl-playground/SKILL.md` |
| Stdlib / host | `.agents/impl-stdlib/SKILL.md` |
| CLI / scripts | `.agents/impl-cli/SKILL.md` |
| Scoped acceptance | `.agents/acceptance-slice-implementer/SKILL.md` |
| Close review | `.agents/reviewer/SKILL.md` |
| Issue closure hygiene | `.agents/verify/SKILL.md` |

## Playground typecheck gap (#500 / #472)

- `playground/src/engine.ts` — `typecheckSource()` returns `parseSource()` diagnostics only.
- Close-gate: `playground/src/tests/typecheck-close-gate.test.ts` expects `phase === "typecheck"` or E02* codes.
- Compiler wasm path exists: `playground/src/compiler-host.ts` runs selfhost wasm with `compile` argv; consider `check` / analysis CLI or LSP diagnostics export.
- **Do not** recreate `crates/ark-playground-wasm`; acceptance prose is stale.

## LSP stdio gap (#634)

- `src/compiler/main/editor.ark` — `cmd_lsp` reads input file path.
- `extensions/arukellt-all-in-one` — `TransportKind.stdio`.
- Lifecycle gates: `scripts/check/check-lsp-lifecycle.py`, `check-dap-lifecycle.py` (script replay may remain).

## Host namespace gating gap (#137)

- Close evidence cited deleted `crates/ark-resolve`; implement in `src/compiler/resolver/`.
- Fixtures: `tests/fixtures/target_gating/t1_import_sockets.ark` expects E0500 on T1 + T3-only import.
- `std/manifest.toml` already lists `std::host::*` modules; missing compile-time target gate.

## Hygiene gap (#418)

- Expected: `scripts/check/check-orphan-inventory.sh` (or manager.py gate equivalent).
- Advisory exit 0; scans docs/tests/benchmarks/artifacts categories per issue acceptance.

## Verification commands (default)

```bash
python3 scripts/manager.py verify quick
```

Issue-specific commands are in each issue's **Required verification** section.

## Wave 2 backlog (publish after wave 1 handoffs)

Dependency-ready, not in wave 1 (path overlap or size):

- #292, #293, #295 — capability / host runtime tests (`src/compiler/wasm/call_host_io.ark`)
- #445, #446, #447 — host family implementations
- #333–#342 LSP feature cluster (sequential on #333)
- #510 — P2 import-table validate gate (emitter.ark; unblocks #074)
- #137 downstream: #138, #139, #63, #136
