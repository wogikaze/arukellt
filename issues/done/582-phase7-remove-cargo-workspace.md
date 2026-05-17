---
Status: done
Created: 2026-04-22
Updated: 2026-05-17
ID: 582
Track: selfhost-retirement
Depends on: 572, 573, 574, 575, 576, 577, 578, 579, 580, 581, 631
Orchestration class: done
Blocks: —
Blocks v5: no
Source: "#529 Phase 7 — Full Rust Deletion (final step)"
Implementation target: "Remove the root Cargo workspace files, the retired crates directory, and reachable Cargo tooling references."
REBUILD_BEFORE_VERIFY: "yes"
CI run URL: n/a (local workspace task; CI-equivalent gates below passed)
---

# 582 — Phase 7 final: remove `Cargo.toml` and `Cargo.lock`

## Summary

The root Cargo workspace was retired. `Cargo.toml`, `Cargo.lock`, and the
`crates/` directory are gone. Reachable scripts, workflows, and `mise.toml`
no longer invoke Cargo or require a Rust toolchain. Current user-facing docs now
describe the selfhost-only architecture.

## Close Evidence

- [x] `test ! -e Cargo.toml && test ! -e Cargo.lock && test ! -d crates` exit 0
- [x] `python scripts/manager.py verify` rc=0: 23 passed, 0 skipped, 0 failed
- [x] `python scripts/manager.py selfhost fixpoint` rc=0: 1 passed, 0 skipped, 0 failed
- [x] `python scripts/manager.py selfhost fixture-parity` rc=0: 1 passed, 0 skipped, 0 failed
- [x] `python scripts/manager.py selfhost parity --mode --cli` rc=0: 1 passed, 0 skipped, 0 failed
- [x] `python scripts/manager.py selfhost diag-parity` rc=0: 1 passed, 0 skipped, 0 failed
- [x] `python scripts/check/check-docs-consistency.py` rc=0: docs consistency OK
- [x] `rg -n "\bcargo\b|Cargo\.toml|crates/|rust-toolchain|\brustc\b" scripts .github/workflows mise.toml` rc=1: no reachable tooling references
- [x] `rg -n "\bcargo\b" scripts/ .github/workflows/ docs/ mise.toml` output cited below; remaining entries are archived docs only

## Upstream Issues Closed

- [x] `issues/done/572-phase7-delete-ark-lsp.md`
- [x] `issues/done/573-phase7-delete-ark-dap.md`
- [x] `issues/done/574-phase7-delete-ark-lexer.md`
- [x] `issues/done/575-phase7-delete-ark-parser.md`
- [x] `issues/done/576-phase7-delete-ark-resolve.md`
- [x] `issues/done/577-phase7-delete-ark-typecheck.md`
- [x] `issues/done/578-phase7-delete-ark-hir.md`
- [x] `issues/done/579-phase7-delete-ark-diagnostics.md`
- [x] `issues/done/580-phase7-delete-ark-manifest.md`
- [x] `issues/done/581-phase7-delete-ark-target.md`
- [x] `issues/done/631-phase7-delete-ark-playground-wasm.md`

## Remaining `cargo` References

These are intentionally retained in archived or historical documentation. No
entry below is part of current scripts, CI, `mise.toml`, README, or
`docs/current-state.md`.

- [x] `docs/migration/v4-to-v5.md:25` — migration note stating the old command is no longer used.
- [x] `docs/migration/v1-to-v2.md:23` — old migration prerequisite text.
- [x] `docs/research/harness/project-comparison.md:30` — research comparison of another project.
- [x] `docs/research/harness/tooling-catalog.md:18` — research comparison of another project.
- [x] `docs/research/harness/tooling-catalog.md:99` — research comparison of another project.
- [x] `docs/research/harness/tooling-catalog.md:134` — research comparison of another project.
- [x] `docs/research/harness/tooling-catalog.md:135` — research comparison of another project.
- [x] `docs/adr/ADR-009-import-syntax.md:27` — ADR comparison with Rust syntax.
- [x] `docs/adr/ADR-031-import-syntax-wit-unification.md:197` — ADR comparison with Rust syntax.
- [x] `docs/adr/ADR-008-component-wrapping.md:45` — historical ADR message text.
- [x] `docs/adr/ADR-017-playground-execution-model.md:203` — historical ADR acceptance table.
- [x] `docs/adr/029-selfhost-native-verification-contract.md:59` — historical ADR context.
- [x] `docs/adr/029-selfhost-native-verification-contract.md:184` — historical ADR context.
- [x] `docs/adr/029-selfhost-native-verification-contract.md:195` — historical ADR context.
- [x] `docs/adr/029-selfhost-native-verification-contract.md:240` — historical ADR context.
- [x] `docs/adr/ADR-028-corehir-lowering-resolution.md:130` — historical ADR work item.
- [x] `docs/adr/ADR-028-corehir-lowering-resolution.md:156` — historical ADR verification command.
- [x] `docs/process/roadmap-v1.md:116` — archived roadmap.
- [x] `docs/process/roadmap-v1.md:119` — archived roadmap.
- [x] `docs/process/roadmap-v1.md:134` — archived roadmap.
- [x] `docs/process/roadmap-v1.md:135` — archived roadmap.
- [x] `docs/process/roadmap-v2.md:133` — archived roadmap.
- [x] `docs/process/roadmap-v2.md:134` — archived roadmap.
- [x] `docs/process/roadmap-v4.md:147` — archived roadmap.
- [x] `docs/process/roadmap-v5.md:166` — archived roadmap.
- [x] `docs/process/benchmark-plan.md:124` — archived benchmark plan.
- [x] `docs/process/benchmark-plan.md:129` — archived benchmark plan.

## False-Done Checklist

1. [x] Workspace files and `crates/` directory are absent.
2. [x] `cargo` search output is cited and remaining entries justified.
3. [x] All upstream issues #572-#581 and #631 are in `issues/done/`.
4. [x] All 4 canonical gates are green with no FAIL or SKIP increase.
5. [x] `python scripts/manager.py verify` rc=0.
6. [x] CI URL is not available in this local workspace; local CI-equivalent gates passed.
7. [x] `docs/current-state.md` and `README.md` reflect selfhost-only architecture.
8. [x] `python scripts/check/check-docs-consistency.py` rc=0.
9. [x] Commit hash is not created in this workspace turn; git status shows scoped retirement changes plus prior task edits.
