# Release guarantees (structured)

> **Generated** from `docs/data/release-guarantees.toml`.
> Normative prose: [`../release-criteria.md`](../release-criteria.md). Checklist: [`../release-checklist.md`](../release-checklist.md).

> **Contract vs current state:** A guarantee is a release-time contract.
> `result` shows the latest observed test outcome (pass/fail/partial/not-run).
> `freshness` shows evidence age (fresh/stale/unknown), derived from `verified_at` + `stale_after_days`.
> A `fail` result means the guarantee is not yet met — it does not remove the guarantee.

## Guarantee matrix

| ID | Tier | Cadence | Summary | Evidence scope | Result | Freshness | Evidence type | Last verified | Known limitation |
|----|------|---------|---------|----------------|--------|-----------|---------------|---------------|------------------|
| `compile_wasm32_gc` | `guaranteed` | `every-pr` | arukellt compile --target wasm32-gc produces valid Wasm (CLI default) | `docs/examples/hello.ark`, `target:wasm32-gc`, `wasm-validation` | ✅ pass | ⏰ stale | smoke | `a80b4181` | — |
| `compile_wasm32` | `guaranteed` | `every-pr` | arukellt compile --target wasm32 produces valid Wasm (supported / AtCoder path) | `docs/examples/hello.ark`, `target:wasm32`, `wasm-validation` | ✅ pass | ⏰ stale | smoke | `a80b4181` | Not the primary CI emphasis; still contract-stable |
| `run_wasmtime` | `guaranteed` | `every-pr` | arukellt run executes via wasmtime | `tests/fixtures/hello_world.ark`, `default-target`, `wasmtime` | ✅ pass | ⏰ stale | smoke | `a80b4181` | — |
| `fixture_harness` | `guaranteed` | `every-pr` | Fixture harness passes for the current observed harness snapshot | `tests/fixtures/manifest.txt`, `registered-harness-cases` | ❌ fail | 🟢 fresh | fixture-set | `982f3102` | See fixture accounting: observed harness ≠ full manifest expansion |
| `determinism` | `guaranteed` | `every-pr` | Same input → identical Wasm bytes | `scripts/check/check-release-determinism.sh::declared-corpus` | ✅ pass | ⏰ stale | smoke | `a80b4181` | — |
| `no_panic_user_paths` | `guaranteed` | `every-pr` | No panic on user-reachable CLI paths | `scripts/check/check-panic-audit.sh::user-reachable-scan` | ✅ pass | ⏰ stale | static-scan | `a80b4181` | — |
| `cli_check` | `guaranteed` | `every-pr` | The check command rejects an invalid source without producing Wasm | `tests/fixtures/diagnostics/type_mismatch.ark`, `diagnostic-exit-status` | ✅ pass | ⏰ stale | smoke | `a80b4181` | Guarantee is command behavior, not acceptance of every valid program |
| `cli_init` | `guaranteed` | `every-pr` | The init command generates every supported project template | `minimal`, `cli`, `with-tests`, `wasi-host` | ✅ pass | ⏰ stale | smoke | `a80b4181` | Template checks validate generated project structure and smoke behavior |
| `cli_doc` | `guaranteed` | `every-pr` | The doc command resolves manifest symbols and generates HTML reference | `std/manifest.toml`, `symbol-lookup`, `html-generation` | ✅ pass | ⏰ stale | smoke | `a80b4181` | Guarantee covers manifest lookup and generator contract |
| `cli_help` | `guaranteed` | `every-pr` | The help command prints the canonical command surface | `src/compiler/main/usage.ark`, `docs/data/cli-surface.toml` | ✅ pass | ⏰ stale | smoke | `a80b4181` | Help output is checked against the structured command catalogue |
| `emit_component` | `provisional` | `nightly` | compile --emit component / component build | `component-emit`, `wasm32-gc` | ⚠️ partial | ⏰ stale | fixture-set, smoke | `a80b4181`, `f2b2a899` | Library exports need s2 wasm; ABI coverage incomplete; may use wasm-tools helpers |
| `lsp` | `provisional` | `nightly` | LSP hover/completion/diagnostics | — | ✅ pass | ⏰ stale | smoke | `a80b4181` | Feature set still evolving |
| `ark_toml` | `provisional` | `nightly` | ark.toml project schema | — | ⬜ not-run | ❓ unknown | smoke | `a80b4181` | Schema fields continue to grow |
| `dap` | `experimental` | `manual` | ark-dap / debug-adapter | — | ⬜ not-run | ❓ unknown | manual | — | Scaffold only |
| `vscode_dap` | `experimental` | `manual` | VS Code extension DAP wiring | — | ⚠️ partial | ⏰ stale | smoke | `a80b4181` | Stub / evolving |
| `freestanding` | `not_guaranteed` | `release-only` | wasm32-freestanding public target | — | ⬜ not-run | ❓ unknown | — | — | ADR-007 retired / hard error |
| `native_targets` | `not_guaranteed` | `release-only` | native-cpp / native-llvm | — | ⬜ not-run | ❓ unknown | — | — | Both targets remain scaffold/partial; native-cpp adds experimental public run (ADR-050) on top of the compiler-private executor ABI (ADR-049); native-llvm stays assembler scaffold |
| `run_native_cpp_experimental` | `experimental` | `ci` | Experimental public `arukellt run --target native-cpp` on Linux x86-64 with clang 14+ (public corpus only) | — | ✅ pass | 🟢 fresh | fixture-set | `local` | Guarantee scope remains tests/fixtures/native_cpp_public + parity/sanitizer (not production-ready; support_tier=scaffold). Full-tree measure v2 readiness gates are COMPLETE per docs/plans/native-cpp-general-backend-readiness.md with documented expected-negative limitations; zero-capture HOF only; no public C ABI/FFI |

## Check catalogue

The release-blocker set is exactly the checks with `release_blocking = true` below.
No supplemental lists.

**Checks vs incidents:** A check is an executable verification command.
An incident is a distinct failure event. Multiple checks may track the
same incident (linked via `incident_id`). Count blockers by distinct
incidents, not by individual checks.

| Check ID | Guarantee | Blocking | In full | In quick | Result | Freshness | Evidence | Affected | Incident | Last verified | Command |
|----------|-----------|:--------:|:-------:|:--------:|--------|-----------|----------|---------:|----------|---------------|---------|
| `check_run_native_cpp_experimental` | `run_native_cpp_experimental` | no | — | — | ✅ pass | 🟢 fresh | `fixture-set` | — | — | `local` | `python3 scripts/check/check-native-cpp-run-promotion-receipt.py && python3 -m unittest scripts.tests.test_native_cpp_runner scripts.tests.test_native_cpp_parity && python3 scripts/check/check-native-cpp-public-sanitizer.py` |
| `check_compile_wasm32_gc` | `compile_wasm32_gc` | 🔴 yes | ✓ | — | ✅ pass | ⏰ stale | `smoke` | — | — | `a80b4181` | `scripts/run/arukellt-selfhost.sh compile docs/examples/hello.ark --target wasm32-gc -o .build/release-checks/wasm32-gc.wasm` |
| `check_compile_wasm32` | `compile_wasm32` | 🔴 yes | ✓ | — | ✅ pass | ⏰ stale | `smoke` | — | — | `a80b4181` | `scripts/run/arukellt-selfhost.sh compile docs/examples/hello.ark --target wasm32 -o .build/release-checks/wasm32.wasm` |
| `check_run_wasmtime` | `run_wasmtime` | 🔴 yes | ✓ | — | ✅ pass | ⏰ stale | `smoke` | — | — | `a80b4181` | `scripts/run/arukellt-selfhost.sh run tests/fixtures/hello_world.ark` |
| `check_fixture_harness` | `fixture_harness` | 🔴 yes | ✓ | — | ❌ fail | 🟢 fresh | `fixture-set` | 1089 | `incident_fixture_parity_1089` | `982f3102` | `python3 scripts/manager.py verify fixtures` |
| `check_determinism` | `determinism` | 🔴 yes | ✓ | — | ✅ pass | ⏰ stale | `smoke` | — | — | `a80b4181` | `bash scripts/check/check-release-determinism.sh` |
| `check_no_panic` | `no_panic_user_paths` | 🔴 yes | ✓ | — | ✅ pass | ⏰ stale | `static-scan` | — | — | `a80b4181` | `bash scripts/check/check-panic-audit.sh` |
| `check_cli_check` | `cli_check` | 🔴 yes | ✓ | — | ✅ pass | ⏰ stale | `smoke` | — | — | `a80b4181` | `python3 scripts/check/check-cli-guarantees.py check` |
| `check_cli_init` | `cli_init` | 🔴 yes | ✓ | — | ✅ pass | ⏰ stale | `smoke` | — | — | `a80b4181` | `python3 scripts/check/check-init-templates.py` |
| `check_cli_doc` | `cli_doc` | 🔴 yes | ✓ | — | ✅ pass | ⏰ stale | `smoke` | — | — | `a80b4181` | `python3 scripts/check/check-manifest-doc.py` |
| `check_cli_help` | `cli_help` | 🔴 yes | ✓ | — | ✅ pass | ⏰ stale | `smoke` | — | — | `a80b4181` | `python3 scripts/check/check-cli-guarantees.py help` |
| `check_close_gate_076` | — | 🔴 yes | — | ✓ | ✅ pass | 🟢 fresh | `smoke` | — | — | `fd14539c23288d3ed993c03600aeed36cd478d06` | `python3 scripts/check/check-false-done-close-gates.py` |
| `check_t3_wasm_validate` | — | 🔴 yes | — | ✓ | ✅ pass | 🟢 fresh | `smoke` | 0 | — | `e18c09aa` | `python3 scripts/check/check-t3-wasm-validate.py` |
| `check_selfhost_fixpoint` | — | 🔴 yes | ✓ | — | ✅ pass | 🟢 fresh | `exhaustive` | 0 | `incident_selfhost_fixpoint` | `fb8a3827` | `python3 scripts/manager.py selfhost fixpoint --build` |
| `check_selfhost_cli_parity` | — | 🔴 yes | ✓ | — | ❌ fail | 🟢 fresh | `smoke` | 2 | `incident_selfhost_cli_parity` | `2cd10f16` | `python3 scripts/manager.py selfhost parity --mode --cli` |
| `check_selfhost_diag_parity` | — | 🔴 yes | ✓ | — | ❌ fail | 🟢 fresh | `smoke` | 3 | `incident_selfhost_diag_parity` | `2cd10f16` | `python3 scripts/manager.py selfhost diag-parity` |
| `check_wat_roundtrip` | — | 🔴 yes | ✓ | — | ✅ pass | 🟢 fresh | `smoke` | 0 | — | `4e07e2a6` | `bash scripts/run/wat-roundtrip.sh` |
| `check_component_interop_wasmtime` | `emit_component` | 🔴 yes | ✓ | — | ✅ pass | 🟢 fresh | `fixture-set` | 0 | — | `f2b2a899` | `python3 scripts/manager.py verify component-interop` |
| `check_opt_equivalence` | — | no | — | ✓ | ✅ pass | ⏰ stale | `smoke` | — | — | `a80b4181` | `bash scripts/run/test-opt-equivalence.sh --quick` |
| `check_binary_version` | — | no | — | — | ✅ pass | ⏰ stale | `smoke` | — | — | `a80b4181` | `arukellt --version` |
| `check_emit_component` | `emit_component` | no | ✓ | — | ⚠️ partial | ⏰ stale | `smoke` | — | — | `a80b4181` | `python3 scripts/check/gate-666-component-library-emit.py` |
| `check_lsp` | `lsp` | no | — | ✓ | ✅ pass | ⏰ stale | `smoke` | — | — | `a80b4181` | `python3 scripts/manager.py verify quick` |
| `check_ark_toml` | `ark_toml` | no | — | — | ⬜ not-run | ❓ unknown | `smoke` | — | — | `a80b4181` | `arukellt build` |
| `check_dap` | `dap` | no | — | — | ⬜ not-run | ❓ unknown | `manual` | — | — | `—` | `manual / scaffold` |
| `check_vscode_dap` | `vscode_dap` | no | — | — | ⚠️ partial | ⏰ stale | `smoke` | — | — | `a80b4181` | `extension-tests (partial)` |


### Stale check derivation details

Stale status is derived from `verified_at` + `stale_after_days` relative to the current date.
Each stale check records the reason and threshold for mechanical verification.

| Check ID | Verified at | Stale after (days) | Stale reason |
|----------|-------------|:------------------:|--------------|
| `check_compile_wasm32_gc` | 2026-07-11 | 30 | — |
| `check_compile_wasm32` | 2026-07-11 | 30 | — |
| `check_run_wasmtime` | 2026-07-11 | 30 | — |
| `check_determinism` | 2026-07-11 | 30 | — |
| `check_no_panic` | 2026-07-11 | 30 | — |
| `check_cli_check` | 2026-07-11 | 30 | — |
| `check_cli_init` | 2026-07-11 | 30 | — |
| `check_cli_doc` | 2026-07-11 | 30 | — |
| `check_cli_help` | 2026-07-11 | 30 | — |
| `check_opt_equivalence` | 2026-07-11 | 30 | — |
| `check_binary_version` | 2026-07-11 | 30 | — |
| `check_emit_component` | 2026-07-11 | 30 | — |
| `check_lsp` | 2026-07-11 | 30 | — |
| `check_vscode_dap` | 2026-07-11 | 30 | — |
