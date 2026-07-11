# Target contract summary (current-state source)

> Hand-maintained source for `docs/current-state.md` target table.
> Not an ADR. Living status belongs here / current-state, not in ADR-007.
>
> Axes (do not collapse into one `status`):
> - **Support Tier**: primary | supported | scaffold | not-started (ADR-007/013)
> - **Implementation**: complete | partial | scaffold | unimplemented
> - **API Stability**: stable | provisional | experimental | unimplemented (ADR-014)
>
> Retired surfaces (e.g. `wasm32-freestanding`) live in `project-state.toml`
> `[[legacy_gaps]]`, not in this public target table.

<!-- BEGIN GENERATED:CURRENT_STATE_TARGET_SUMMARY_SOURCE -->
| Target | Support Tier | Implementation | API Stability | Run | Notes |
|--------|--------------|----------------|---------------|-----|-------|
| `wasm32` | supported | complete | stable | Yes | AtCoder / linear-memory competition path (canonical; was `wasm32-wasi-p1`) |
| `wasm32-gc` | primary | partial | stable | Yes | Primary (ADR-013): Wasm GC + WASI P2 default host profile; GC lowering still partial |
| `native-cpp` | scaffold | scaffold | experimental | No | Scaffold C99 emit path |
| `native-llvm` | scaffold | scaffold | experimental | No | Scaffold LLVM IR emit; semantics/ABI per ADR-045 undecided |
| `wasm32-gc` + `--wasi p3` | not-started | unimplemented | unimplemented | No | Host profile on same language target; not a separate primary |
<!-- END GENERATED:CURRENT_STATE_TARGET_SUMMARY_SOURCE -->
