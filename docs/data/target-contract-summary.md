# Target contract summary (current-state source)

> Hand-maintained source for `docs/current-state.md` target table.
> Not an ADR. Living status belongs here / current-state, not in ADR-007.

<!-- BEGIN GENERATED:CURRENT_STATE_TARGET_SUMMARY_SOURCE -->
| Target | Tier | ADR-013 Tier | Status | Run | Notes |
|--------|------|--------------|--------|-----|-------|
| `wasm32` | — | supported | stable | Yes | AtCoder / linear-memory competition path (canonical; was `wasm32-wasi-p1`) |
| `wasm32-gc` | — | primary | stable | Yes | Primary (ADR-013): Wasm GC + WASI P2 default host profile |
| `native-cpp` | — | scaffold | scaffold | No | Scaffold C99 emit path |
| `native-llvm` | — | scaffold | scaffold | No | Scaffold LLVM IR emit; semantics/ABI per ADR-045 undecided |
| `wasm32-gc` + `--wasi p3` | — | not-started | not-started | No | Host profile on same language target; not a separate primary |
<!-- END GENERATED:CURRENT_STATE_TARGET_SUMMARY_SOURCE -->
