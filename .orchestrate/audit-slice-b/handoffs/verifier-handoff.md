# Verifier handoff — audit-slice-b

Date: 2026-06-12

## verify quick

- Without wasmtime/arukellt: 143/149 passed, 6 failed
- With wasmtime + `arukellt` symlink to `scripts/run/arukellt-selfhost.sh`: 146/149 passed, 3 failed
  - docs consistency (generated docs drift)
  - false-done hygiene FD-02 (#487 Status: fixed in issues/done/)
  - doc example check (cookbook/module-system blocks: file open errors under target/tmp/)

Slice B commit `c5437206` touched only `issues/**`, audit report, orchestrate state — no new verify failures attributable to reopen moves.

## Reopen spot-checks (7/7 confirmed)

| ID | Check | Result |
|----|-------|--------|
| 464 | `src/compiler/main/init.ark` — single scaffold; no template flags | confirmed |
| 456 | `src/compiler/main/doc.ark` — markdown reader only | confirmed |
| 491 | `playground-ci.yml` — no `playground-wasm-size` job | confirmed |
| 216 | LSP capabilities: hover/definition/completion only; no formatting | confirmed |
| 217 | `dispatch_features.ark` — no codeAction handler | confirmed |
| 219 | no signatureHelp/inlayHint/foldingRange in LSP | confirmed |
| 440 | no fixAll/codeAction in lsp/ or extensions/ | confirmed |

All 7 absent from `issues/done/`; present in `issues/open/` with reopen sections and unchecked acceptance.

## Truly-done spot-checks (sample)

- `docs/playground/index.html`, `compiler-asset.json` exist
- `playground-ci.yml` has `playground-lighthouse` job
- `pages.yml` deploys `docs/**`
- `dispatch_aux.ark` wires init/doc/fmt
- `extensions/.../extension.test.js` exists
- Wave 4 appended to `docs/process/false-done-audit-2026-06-12.md`

## Live CLI probes

- `arukellt init --list-templates` → exit 1 (no subcommand flag)
- `arukellt init --help` → top-level help only; no template options
- `arukellt doc --help` → same; `--json` is global diagnostic flag, not doc output
