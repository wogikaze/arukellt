<!-- orchestrate handoff
task: verify-wave1
status: partial
-->

## Wave 1 verification summary

### Closed issues
- **#418** — orphan/stale inventory script added and registered in verify quick
- **#137** — T3-only `std::host::*` target gating in selfhost resolver
- **#500** — playground typecheck export path via selfhost wasm `check --json`
- **#472** — playground product claim satisfied (type-phase diagnostics)

### Open / partial
- **#634** — stdio LSP/DAP entrypoint implemented in `src/compiler/main/editor.ark` but lifecycle stdio gates fail because `resolve_ide_gate_compiler_wasm` uses pinned `bootstrap/arukellt-selfhost.wasm` without stdio support. Rebuilding `.build/selfhost/arukellt-s2.wasm` blocked by missing modules: `resolver_register_wit`, `mir_lower_wit_ctx_init`, `mir_lower_body_call_wit`, `typechecker_module_wit`.

### verify quick
- **147/150** passed
- Failures: false-done hygiene (#487 status — fixed in follow-up commit), LSP lifecycle stdio, DAP lifecycle stdio

### Orchestration infra
- `CURSOR_API_KEY` unset — `bun cli.ts run` cannot spawn cloud workers; wave executed via in-session agents.

### Wave 2 recommendation
1. Restore missing WIT helper modules to unblock selfhost s2 rebuild → close #634
2. #292, #293, #295 host capability cluster
3. #510 P2 import-table validate gate → #074
4. #333 LSP navigation cluster
