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
- **#634** — stdio dispatch wired in `editor.ark`; script-replay LSP/DAP lifecycle passes on rebuilt `arukellt-s2.wasm`. Bare stdio path traps in `stdio::read_to_string()` / `__intrinsic_stdin_read_to_string` (wasm unreachable). WIT stub modules added to unblock s2 rebuild; stdin read fix remains open.

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
