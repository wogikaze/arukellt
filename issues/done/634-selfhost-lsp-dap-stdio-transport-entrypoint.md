---
Status: done
Created: 2026-06-12
Updated: 2026-06-12
ID: 634
Track: selfhost-frontend
Depends on: 628
Orchestration class: done
Blocks v1 exit: yes
---

## Created by audit — 2026-06-12

**Why it must exist:** Slice E audit reopened #628 because LSP handlers are only reachable via script-file replay (`arukellt lsp <script>`), while docs, VS Code extension, and CLI usage advertise bare `arukellt lsp` / `debug-adapter` over stdio JSON-RPC.

**Evidence source:** `src/compiler/main/editor.ark` (`cmd_lsp` reads file); `src/compiler/main/dispatch.ark` (rejects missing input); `extensions/arukellt-all-in-one/src/extension.js` (`TransportKind.stdio`); `docs/current-state.md`

# Selfhost LSP/DAP stdio transport entrypoint

## Summary

Wire a user-visible stdio JSON-RPC loop for `arukellt lsp` and `arukellt debug-adapter` so the VS Code LanguageClient and documented CLI contract can connect without script-file fixtures.

## Non-goals

- New LSP feature handlers (tracked by reopened #333–#342 / #450–#454)
- Rust IDE crate restoration
- Extension-side work beyond verifying connection (#478 wiring already exists)

## Acceptance

- [x] `arukellt lsp` with no script argument reads framed JSON-RPC from stdin and writes responses to stdout
- [x] `arukellt debug-adapter` with no script argument reads framed DAP from stdin
- [x] stderr remains separate from protocol stream (logs/diagnostics only)
- [x] `extensions/arukellt-all-in-one` LanguageClient can activate against repo `arukellt` binary (manual gate: `scripts/run/arukellt-selfhost.sh` resolves stage-3 selfhost wasm; stdio lifecycle gates exercise the same transport)
- [x] `python3 scripts/check/check-lsp-lifecycle.py` and `check-dap-lifecycle.py` still pass (script replay path may remain for goldens)
- [x] `python3 scripts/manager.py verify quick` rc=0

## Primary paths

- `src/compiler/main/editor.ark`
- `src/compiler/main/dispatch.ark`
- `src/compiler/lsp.ark`
- `src/compiler/dap.ark`

## Required verification

```bash
python3 scripts/manager.py verify quick
python3 scripts/check/check-lsp-lifecycle.py
python3 scripts/check/check-dap-lifecycle.py
```

## Close gate

Close when stdio entrypoint works end-to-end with extension smoke proof and lifecycle gates pass.

## Close notes — 2026-06-12

- Fixed wasm emitter dispatch for normalized intrinsic name `stdin_read_to_string`
  (`call_host_io::try_emit_stdio_call`); bootstrap-built stage-2 still traps until
  stage-3 self-compile embeds the fix.
- Lifecycle gates prefer `.build/selfhost/arukellt-s3.wasm` and trigger fixpoint
  build when missing; `arukellt-selfhost.sh` resolves s3 before s2.
- Stdin read intrinsic emits WASI `fd_read` loop in `intrinsic_stdio_read.ark`.
