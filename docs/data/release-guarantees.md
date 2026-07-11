# Release guarantees (structured)

> **Generated** from `docs/data/release-guarantees.toml`.
> Normative prose: [`../release-criteria.md`](../release-criteria.md). Checklist: [`../release-checklist.md`](../release-checklist.md).

> **Contract vs current state:** A guarantee is a release-time contract.
> `current_status` shows the latest observed verification result, which may be `fail`.
> A `fail` status means the guarantee is not yet met — it does not remove the guarantee.

## Guarantee matrix

| ID | Tier | Summary | Evidence scope | Current status | Evidence type | Last verified | Known limitation |
|----|------|---------|----------------|----------------|---------------|---------------|------------------|
| `compile_wasm32_gc` | `guaranteed` | arukellt compile --target wasm32-gc produces valid Wasm (CLI default) | `docs/examples/hello.ark`, `target:wasm32-gc`, `wasm-validation` | ✅ pass | smoke | `a80b4181` | — |
| `compile_wasm32` | `guaranteed` | arukellt compile --target wasm32 produces valid Wasm (supported / AtCoder path) | `docs/examples/hello.ark`, `target:wasm32`, `wasm-validation` | ✅ pass | smoke | `a80b4181` | Not the primary CI emphasis; still contract-stable |
| `run_wasmtime` | `guaranteed` | arukellt run executes via wasmtime | `tests/fixtures/hello_world.ark`, `default-target`, `wasmtime` | ✅ pass | smoke | `a80b4181` | — |
| `fixture_harness` | `guaranteed` | Fixture harness passes for the current observed harness snapshot | `tests/fixtures/manifest.txt`, `registered-harness-cases` | ❌ fail | fixture-set | `89eb5eb4` | See fixture accounting: observed harness ≠ full manifest expansion |
| `determinism` | `guaranteed` | Same input → identical Wasm bytes | `scripts/check/check-release-determinism.sh::declared-corpus` | ✅ pass | smoke | `a80b4181` | — |
| `no_panic_user_paths` | `guaranteed` | No panic on user-reachable CLI paths | `scripts/check/check-panic-audit.sh::user-reachable-scan` | ✅ pass | static-scan | `a80b4181` | — |
| `cli_check` | `guaranteed` | The check command rejects an invalid source without producing Wasm | `tests/fixtures/diagnostics/type_mismatch.ark`, `diagnostic-exit-status` | ✅ pass | smoke | `a80b4181` | Guarantee is command behavior, not acceptance of every valid program |
| `cli_init` | `guaranteed` | The init command generates every supported project template | `minimal`, `cli`, `with-tests`, `wasi-host` | ✅ pass | smoke | `a80b4181` | Template checks validate generated project structure and smoke behavior |
| `cli_doc` | `guaranteed` | The doc command resolves manifest symbols and generates HTML reference | `std/manifest.toml`, `symbol-lookup`, `html-generation` | ✅ pass | smoke | `a80b4181` | Guarantee covers manifest lookup and generator contract |
| `cli_help` | `guaranteed` | The help command prints the canonical command surface | `src/compiler/main/usage.ark`, `docs/data/cli-surface.toml` | ✅ pass | smoke | `a80b4181` | Help output is checked against the structured command catalogue |
| `emit_component` | `provisional` | compile --emit component / component build | `component-emit`, `wasm32-gc` | ❌ fail | fixture-set, smoke | `a80b4181` | Library exports need s2 wasm; ABI coverage incomplete; may use wasm-tools helpers |
| `lsp` | `provisional` | LSP hover/completion/diagnostics | — | ✅ pass | smoke | `a80b4181` | Feature set still evolving |
| `ark_toml` | `provisional` | ark.toml project schema | — | ⏰ stale | smoke | `a80b4181` | Schema fields continue to grow |
| `dap` | `experimental` | ark-dap / debug-adapter | — | ⬜ not-run | manual | — | Scaffold only |
| `vscode_dap` | `experimental` | VS Code extension DAP wiring | — | ⏰ stale | smoke | `a80b4181` | Stub / evolving |
| `freestanding` | `not_guaranteed` | wasm32-freestanding public target | — | ⬜ not-run | — | — | ADR-007 retired / hard error |
| `native_targets` | `not_guaranteed` | native-cpp / native-llvm | — | ⬜ not-run | — | — | Scaffold; ABI undecided (ADR-045) |

## Check catalogue

The release-blocker set is exactly the checks with `release_blocking = true` below.
No supplemental lists.

**Checks vs incidents:** A check is an executable verification command.
An incident is a distinct failure event. Multiple checks may track the
same incident (linked via `incident_id`). Count blockers by distinct
incidents, not by individual checks.

| Check ID | Guarantee | Blocking | In full | In quick | Current | Evidence | Affected | Incident | Last verified | Command |
|----------|-----------|:--------:|:-------:|:--------:|---------|----------|---------:|----------|---------------|---------|
| `check_compile_wasm32_gc` | `compile_wasm32_gc` | 🔴 yes | ✓ | — | ✅ pass | `smoke` | — | — | `a80b4181` | `scripts/run/arukellt-selfhost.sh compile docs/examples/hello.ark --target wasm32-gc -o .build/release-checks/wasm32-gc.wasm` |
| `check_compile_wasm32` | `compile_wasm32` | 🔴 yes | ✓ | — | ✅ pass | `smoke` | — | — | `a80b4181` | `scripts/run/arukellt-selfhost.sh compile docs/examples/hello.ark --target wasm32 -o .build/release-checks/wasm32.wasm` |
| `check_run_wasmtime` | `run_wasmtime` | 🔴 yes | ✓ | — | ✅ pass | `smoke` | — | — | `a80b4181` | `scripts/run/arukellt-selfhost.sh run tests/fixtures/hello_world.ark` |
| `check_fixture_harness` | `fixture_harness` | 🔴 yes | ✓ | — | ❌ fail | `fixture-set` | 367 | `incident_fixture_parity_367` | `89eb5eb4` | `python3 scripts/manager.py verify fixtures` |
| `check_determinism` | `determinism` | 🔴 yes | ✓ | — | ✅ pass | `smoke` | — | — | `a80b4181` | `bash scripts/check/check-release-determinism.sh` |
| `check_no_panic` | `no_panic_user_paths` | 🔴 yes | ✓ | — | ✅ pass | `static-scan` | — | — | `a80b4181` | `bash scripts/check/check-panic-audit.sh` |
| `check_cli_check` | `cli_check` | 🔴 yes | ✓ | — | ✅ pass | `smoke` | — | — | `a80b4181` | `python3 scripts/check/check-cli-guarantees.py check` |
| `check_cli_init` | `cli_init` | 🔴 yes | ✓ | — | ✅ pass | `smoke` | — | — | `a80b4181` | `python3 scripts/check/check-init-templates.py` |
| `check_cli_doc` | `cli_doc` | 🔴 yes | ✓ | — | ✅ pass | `smoke` | — | — | `a80b4181` | `python3 scripts/check/check-manifest-doc.py` |
| `check_cli_help` | `cli_help` | 🔴 yes | ✓ | — | ✅ pass | `smoke` | — | — | `a80b4181` | `python3 scripts/check/check-cli-guarantees.py help` |
| `check_close_gate_076` | — | 🔴 yes | — | ✓ | ❌ fail | `smoke` | 1 | — | `89eb5eb4` | `python3 scripts/check/check-false-done-close-gates.py` |
| `check_selfhost_fixpoint` | — | 🔴 yes | ✓ | — | ❌ fail | `exhaustive` | 1 | — | `a80b4181` | `python3 scripts/manager.py selfhost fixpoint --build` |
| `check_selfhost_cli_parity` | — | 🔴 yes | ✓ | — | ❌ fail | `smoke` | 3 | — | `a80b4181` | `python3 scripts/manager.py selfhost parity --mode --cli` |
| `check_selfhost_diag_parity` | — | 🔴 yes | ✓ | — | ✅ pass | `smoke` | — | — | `a80b4181` | `python3 scripts/manager.py selfhost diag-parity` |
| `check_wat_roundtrip` | — | 🔴 yes | ✓ | — | ❌ fail | `smoke` | 1 | — | `a80b4181` | `bash scripts/run/wat-roundtrip.sh` |
| `check_component_interop_wasmtime` | `emit_component` | 🔴 yes | ✓ | — | ❌ fail | `fixture-set` | 103 | `incident_component_interop_103` | `a80b4181` | `python3 scripts/manager.py verify --component-interop` |
| `check_opt_equivalence` | — | no | — | ✓ | ✅ pass | `smoke` | — | — | `a80b4181` | `bash scripts/run/test-opt-equivalence.sh --quick` |
| `check_binary_version` | — | no | — | — | ✅ pass | `smoke` | — | — | `a80b4181` | `arukellt --version` |
| `check_emit_component` | `emit_component` | no | ✓ | — | ⏰ stale | `smoke` | — | — | `a80b4181` | `python3 scripts/manager.py verify --component` |
| `check_lsp` | `lsp` | no | — | ✓ | ✅ pass | `smoke` | — | — | `a80b4181` | `python3 scripts/manager.py verify quick` |
| `check_ark_toml` | `ark_toml` | no | — | — | ⏰ stale | `smoke` | — | — | `a80b4181` | `arukellt build` |
| `check_dap` | `dap` | no | — | — | ⬜ not-run | `manual` | — | — | `—` | `manual / scaffold` |
| `check_vscode_dap` | `vscode_dap` | no | — | — | ⏰ stale | `smoke` | — | — | `a80b4181` | `extension-tests (partial)` |
