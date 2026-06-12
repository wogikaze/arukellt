# Wave 1 — priority open issues

Ordered by dependency index position + audit priority lanes. Workers should close issues only when acceptance is met and `verify quick` passes.

## Parallel batch (no path conflicts)

1. **#634** — Selfhost LSP/DAP stdio JSON-RPC entrypoint (v1 lane)
2. **#500 + #472** — Playground real typecheck (500 is prerequisite for 472)
3. **#418** — Orphan/stale file inventory script
4. **#137** — `std::host::*` target-gated import diagnostics in selfhost resolver

## Sequential follow-ups (wave 2 planner)

| After | Next |
|-------|------|
| #418 | #422 artifact size budget |
| #500/#472 | #491 CI perf budget gates |
| #137 | #138, #139, #63, #136 host rollout |
| #446, #447 | #633 manifest honesty |
| #510 | #074, #076 WASI P2 native |
| #333 | #334–#342, #439–#441 LSP navigation cluster |

## Issues explicitly deferred (blocked upstream)

- #124, #28 — WIT import syntax / jco
- #474, #476, #443, #618 — component compose / async
- #118 — needs #117 + #074
- #60, #62 — needs #510 + #074
- #633 — needs #446 + #447 (or docs-only honesty slice without impl)
