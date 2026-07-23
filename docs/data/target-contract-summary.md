# Target / host contract summary

> **Generated** from `docs/data/project-state.toml` by `scripts/gen/generate-docs.py`.
> Do not hand-edit the tables below. Edit `project-state.toml` instead.
>
> Axes:
> - **Support Tier**: primary | supported | scaffold | not-started (ADR-007/013)
> - **Implementation**: complete | partial | scaffold | unimplemented
> - **Contract Stability**: stable | provisional | experimental (never `unimplemented`)
>
> Host profiles are separate from language targets.

<!-- BEGIN GENERATED:CURRENT_STATE_TARGET_SUMMARY_SOURCE -->
| Target | Support Tier | Implementation | Contract Stability | Run | Notes |
|--------|--------------|----------------|--------------------|-----|-------|
| `wasm32` | supported | complete | stable | Yes | Supported: AtCoder / linear-memory competition path |
| `wasm32-gc` | primary | partial | stable | Yes | Primary (ADR-013): Wasm GC + WASI P2 default host profile; GC lowering still partial |
| `native-cpp` | scaffold | partial | experimental | No | Recovered C99 emitter/runtime + native-executor lane; S2/S3 equality+determinism green; --allow-high-rss operational (arena ~12 GiB / warm ~3.8 min); GC=1 RSS ~1.55 GiB but warm ~8 min (experimental wall pending) |
| `native-llvm` | scaffold | scaffold | experimental | No | Scaffold LLVM IR emit; ADR-049 decides native-cpp only, native-llvm remains undecided |
<!-- END GENERATED:CURRENT_STATE_TARGET_SUMMARY_SOURCE -->

<!-- BEGIN GENERATED:HOST_PROFILE_SUMMARY_SOURCE -->
| Host profile | Targets | Planned | Rejected | Support Tier | Implementation | Contract Stability | Notes |
|--------------|---------|---------|----------|--------------|----------------|--------------------|-------|
| `wasi-p1` | `wasm32` | — | `wasm32-gc` | supported | partial | stable | WASI Preview 1 host profile (AtCoder / linear path on wasm32 only; wasm32-gc+P1 rejected per ADR-007) |
| `wasi-p2` | `wasm32-gc` | — | — | primary | partial | stable | Default host profile for primary target wasm32-gc (ADR-013) |
| `wasi-p3` | `wasm32-gc` | — | — | not-started | unimplemented | experimental | Future WASI Preview 3 host profile on wasm32-gc; not a separate language target |
<!-- END GENERATED:HOST_PROFILE_SUMMARY_SOURCE -->
