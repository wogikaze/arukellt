---
Status: open
Created: 2026-06-12
Updated: 2026-06-12
ID: 634
Track: selfhost-frontend
Depends on: 628
Orchestration class: implementation-ready
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

- [ ] `arukellt lsp` with no script argument reads framed JSON-RPC from stdin and writes responses to stdout
- [ ] `arukellt debug-adapter` with no script argument reads framed DAP from stdin
- [ ] stderr remains separate from protocol stream (logs/diagnostics only)
- [ ] `extensions/arukellt-all-in-one` LanguageClient can activate against repo `arukellt` binary (smoke test or documented manual gate)
- [ ] `python3 scripts/check/check-lsp-lifecycle.py` and `check-dap-lifecycle.py` still pass (script replay path may remain for goldens)
- [ ] `python3 scripts/manager.py verify quick` rc=0

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

## Implementation progress — 2026-06-12

- Wired bare `arukellt lsp` / `arukellt debug-adapter` dispatch to allow missing
  input and read framed protocol bytes from stdin.
- Preserved existing script-file replay path for lifecycle golden fixtures.
- Added `std::host::stdio::read_to_string()` and a selfhost Wasm stdin read-all
  intrinsic backed by WASI `fd_read` / T2 `arukellt_io.read`.
- Extended `check-lsp-lifecycle.py` and `check-dap-lifecycle.py` to replay each
  golden both through the script path and the bare stdio path.

Verification is not yet closeable on this branch: rebuilding a current
`.build/selfhost/arukellt-s2.wasm` is blocked by pre-existing missing WIT helper
modules (`resolver_register_wit`, `mir_lower_wit_ctx_init`,
`mir_lower_body_call_wit`, `typechecker_module_wit`; also noted in #137 close
notes). With no current s2 artifact, the lifecycle scripts fall back to the
pinned bootstrap compiler, which still rejects bare stdio with
`error: no input file specified`.
