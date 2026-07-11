# Release guarantees (structured)

> **Generated** from `docs/data/release-guarantees.toml`.
> Normative prose: [`../release-criteria.md`](../release-criteria.md). Checklist: [`../release-checklist.md`](../release-checklist.md).

| ID | Tier | Summary | Check | CI job | Blocker | Known limitation |
|----|------|---------|-------|--------|:-------:|------------------|
| `compile_wasm32_gc` | `guaranteed` | arukellt compile --target wasm32-gc produces valid Wasm (CLI default) | `scripts/run/arukellt-selfhost.sh compile docs/examples/hello.ark --target wasm32-gc -o .build/release-checks/wasm32-gc.wasm` | `verification` | yes | — |
| `compile_wasm32` | `guaranteed` | arukellt compile --target wasm32 produces valid Wasm (supported / AtCoder path) | `scripts/run/arukellt-selfhost.sh compile docs/examples/hello.ark --target wasm32 -o .build/release-checks/wasm32.wasm` | `verification` | yes | Not the primary CI emphasis; still contract-stable |
| `run_wasmtime` | `guaranteed` | arukellt run executes via wasmtime | `scripts/run/arukellt-selfhost.sh run tests/fixtures/hello_world.ark` | `verification` | yes | — |
| `fixture_harness` | `guaranteed` | Fixture harness passes for the current observed harness snapshot | `python3 scripts/manager.py verify fixtures` | `verification` | yes | See fixture accounting: observed harness ≠ full manifest expansion |
| `determinism` | `guaranteed` | Same input → identical Wasm bytes | `bash scripts/check/check-release-determinism.sh` | `verification` | yes | No dedicated top-level ci.yml job |
| `no_panic_user_paths` | `guaranteed` | No panic on user-reachable CLI paths | `bash scripts/check/check-panic-audit.sh` | `verification` | yes | — |
| `emit_component` | `provisional` | compile --emit component / component build | `python3 scripts/manager.py verify --component` | `verification` | no | Library exports need s2 wasm; ABI coverage incomplete; may use wasm-tools helpers |
| `lsp` | `provisional` | LSP hover/completion/diagnostics | `python3 scripts/manager.py verify quick (LSP gates)` | `verification` | no | Feature set still evolving |
| `ark_toml` | `provisional` | ark.toml project schema | `arukellt build` | `verification` | no | Schema fields continue to grow |
| `dap` | `experimental` | ark-dap / debug-adapter | `manual / scaffold` | `none` | no | Scaffold only |
| `vscode_dap` | `experimental` | VS Code extension DAP wiring | `extension-tests (partial)` | `extension-tests` | no | Stub / evolving |
| `freestanding` | `not_guaranteed` | wasm32-freestanding public target | `n/a` | `none` | no | ADR-007 retired / hard error |
| `native_targets` | `not_guaranteed` | native-cpp / native-llvm | `n/a` | `none` | no | Scaffold; ABI undecided (ADR-045) |
