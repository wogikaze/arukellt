# Release guarantees (structured)

> **Generated** from `docs/data/release-guarantees.toml`.
> Normative prose: [`../release-criteria.md`](../release-criteria.md). Checklist: [`../release-checklist.md`](../release-checklist.md).

| ID | Tier | Summary | Evidence scope | Check | CI job | Blocker | Known limitation |
|----|------|---------|----------------|-------|--------|:-------:|------------------|
| `compile_wasm32_gc` | `guaranteed` | arukellt compile --target wasm32-gc produces valid Wasm (CLI default) | `docs/examples/hello.ark`, `target:wasm32-gc`, `wasm-validation` | `scripts/run/arukellt-selfhost.sh compile docs/examples/hello.ark --target wasm32-gc -o .build/release-checks/wasm32-gc.wasm` | `verification` | yes | — |
| `compile_wasm32` | `guaranteed` | arukellt compile --target wasm32 produces valid Wasm (supported / AtCoder path) | `docs/examples/hello.ark`, `target:wasm32`, `wasm-validation` | `scripts/run/arukellt-selfhost.sh compile docs/examples/hello.ark --target wasm32 -o .build/release-checks/wasm32.wasm` | `verification` | yes | Not the primary CI emphasis; still contract-stable |
| `run_wasmtime` | `guaranteed` | arukellt run executes via wasmtime | `tests/fixtures/hello_world.ark`, `default-target`, `wasmtime` | `scripts/run/arukellt-selfhost.sh run tests/fixtures/hello_world.ark` | `verification` | yes | — |
| `fixture_harness` | `guaranteed` | Fixture harness passes for the current observed harness snapshot | `tests/fixtures/manifest.txt`, `registered-harness-cases` | `python3 scripts/manager.py verify fixtures` | `verification` | yes | See fixture accounting: observed harness ≠ full manifest expansion |
| `determinism` | `guaranteed` | Same input → identical Wasm bytes | `scripts/check/check-release-determinism.sh::declared-corpus` | `bash scripts/check/check-release-determinism.sh` | `verification` | yes | No dedicated top-level ci.yml job |
| `no_panic_user_paths` | `guaranteed` | No panic on user-reachable CLI paths | `scripts/check/check-panic-audit.sh::user-reachable-scan` | `bash scripts/check/check-panic-audit.sh` | `verification` | yes | — |
| `cli_check` | `guaranteed` | The check command rejects an invalid source without producing Wasm | `tests/fixtures/diagnostics/type_mismatch.ark`, `diagnostic-exit-status` | `python3 scripts/check/check-cli-guarantees.py check` | `verification` | yes | Guarantee is command behavior, not acceptance of every valid program |
| `cli_init` | `guaranteed` | The init command generates every supported project template | `minimal`, `cli`, `with-tests`, `wasi-host` | `python3 scripts/check/check-init-templates.py` | `verification` | yes | Template checks validate generated project structure and smoke behavior |
| `cli_doc` | `guaranteed` | The doc command resolves manifest symbols and generates HTML reference | `std/manifest.toml`, `symbol-lookup`, `html-generation` | `python3 scripts/check/check-manifest-doc.py` | `verification` | yes | Guarantee covers manifest lookup and generator contract |
| `cli_help` | `guaranteed` | The help command prints the canonical command surface | `src/compiler/main/usage.ark`, `docs/data/cli-surface.toml` | `python3 scripts/check/check-cli-guarantees.py help` | `verification` | yes | Help output is checked against the structured command catalogue |
| `emit_component` | `provisional` | compile --emit component / component build | — | `python3 scripts/manager.py verify --component` | `verification` | no | Library exports need s2 wasm; ABI coverage incomplete; may use wasm-tools helpers |
| `lsp` | `provisional` | LSP hover/completion/diagnostics | — | `python3 scripts/manager.py verify quick (LSP gates)` | `verification` | no | Feature set still evolving |
| `ark_toml` | `provisional` | ark.toml project schema | — | `arukellt build` | `verification` | no | Schema fields continue to grow |
| `dap` | `experimental` | ark-dap / debug-adapter | — | `manual / scaffold` | `none` | no | Scaffold only |
| `vscode_dap` | `experimental` | VS Code extension DAP wiring | — | `extension-tests (partial)` | `extension-tests` | no | Stub / evolving |
| `freestanding` | `not_guaranteed` | wasm32-freestanding public target | — | `n/a` | `none` | no | ADR-007 retired / hard error |
| `native_targets` | `not_guaranteed` | native-cpp / native-llvm | — | `n/a` | `none` | no | Scaffold; ABI undecided (ADR-045) |
