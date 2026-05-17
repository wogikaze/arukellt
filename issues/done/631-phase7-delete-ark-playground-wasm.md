---
Status: done
Created: 2026-05-17
Updated: 2026-05-17
ID: 631
Track: selfhost-retirement
Depends on: 564
Orchestration class: completed
Blocks: 574, 575, 576, 577, 582
Blocks v5: no
Source: "#630 Phase 7 sequencing fix — `ark-playground-wasm` consumes Rust frontend crates and must be retired before deleting those crates."
Implementation target: "Per #529 Phase 7, remove the Rust `crates/ark-playground-wasm` package and migrate or retire any playground build/docs/CI references that require it."
REBUILD_BEFORE_VERIFY: "yes (workspace topology change)"
---

# 631 — Phase 7: Delete `crates/ark-playground-wasm`

## Summary

`crates/ark-playground-wasm` is a downstream Rust consumer of `ark-lexer`,
`ark-parser`, `ark-resolve`, `ark-typecheck`, and `ark-diagnostics`. Phase 7
cannot safely delete those core crates while the playground wasm crate remains
in the workspace. This issue makes the missing consumer-deletion step explicit.

Closed 2026-05-17: the browser playground now uses
`playground/src/engine.ts`, a TypeScript engine that preserves parse, format,
tokenize, typecheck, and version response surfaces without depending on the
retired Rust wasm crate.

## Acceptance

- [x] `crates/ark-playground-wasm/` directory removed (`[ ! -d crates/ark-playground-wasm ]`)
- [x] Workspace `Cargo.toml` no longer lists `crates/ark-playground-wasm`
- [x] `Cargo.lock` contains no `name = "ark-playground-wasm"`
- [x] `.github/workflows/pages.yml` and `.github/workflows/playground-ci.yml` no longer run `wasm-pack` in `crates/ark-playground-wasm`
- [x] Docs that name `crates/ark-playground-wasm` are updated to the replacement playground wasm/source path or marked historical
- [x] `python scripts/manager.py verify quick` passes
- [x] 4 canonical selfhost gates remain green with FAIL=0 and no SKIP increase

## Close Notes

- Added `playground/src/engine.ts` as the browser-native playground source path.
- Updated main-thread and worker playground clients to call the TypeScript
  engine instead of wasm-pack glue.
- Removed `crates/ark-playground-wasm` from the Cargo workspace and regenerated
  `Cargo.lock`.
- Removed wasm-pack build steps from playground Pages/CI workflows.
- Updated generated and manual playground docs to describe the TypeScript engine
  and mark the Rust Wasm path historical.
- `rg -l "\bark_playground_wasm\b|\bark-playground-wasm\b|crates/ark-playground-wasm" Cargo.toml Cargo.lock crates/ scripts/ docs/ .github/` returned no files.

## Verification Results

- `python scripts/manager.py verify quick`: PASS (23 passed, 0 skipped, 0 failed)
- `python scripts/manager.py selfhost fixpoint`: PASS (1 passed, 0 skipped, 0 failed)
- `python scripts/manager.py selfhost fixture-parity`: PASS (1 passed, 0 skipped, 0 failed)
- `python scripts/manager.py selfhost parity --mode --cli`: PASS (1 passed, 0 skipped, 0 failed)
- `python scripts/manager.py selfhost diag-parity`: PASS (1 passed, 0 skipped, 0 failed)
- `cargo check --workspace`: PASS
- `npm run build`: PASS
- `npm run typecheck`: PASS
- `npm test`: PASS (7 tests passed)
- `bash scripts/check/check-playground-size.sh --bundle-dir docs/playground/dist`: PASS (104 KB / 512 KB)

## Required Verification

```bash
python scripts/manager.py verify quick
python scripts/manager.py selfhost fixpoint
python scripts/manager.py selfhost fixture-parity
python scripts/manager.py selfhost parity --mode --cli
python scripts/manager.py selfhost diag-parity
cargo check --workspace
rg -l "\bark_playground_wasm\b|\bark-playground-wasm\b|crates/ark-playground-wasm" Cargo.toml Cargo.lock crates/ scripts/ docs/ .github/
```

## STOP_IF

- A replacement browser playground wasm build path is not defined and the user-facing playground would lose its only compiler bridge.
- Removing the crate causes any canonical selfhost gate to regress.
- CI still requires `wasm-pack` output from this crate after deletion.

## Blocks

- #574 (`ark-lexer`)
- #575 (`ark-parser`)
- #576 (`ark-resolve`)
- #577 (`ark-typecheck`)
- #582 (final Cargo workspace removal)
