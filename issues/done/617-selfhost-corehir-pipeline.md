---
Status: done
Created: 2026-04-25
Updated: 2026-05-17
ID: 617
Track: pipeline-refactor
Depends on: 529
Orchestration class: done
Orchestration upstream: None
Blocks v4 exit: True
# Selfhost Compiler: "Implement CoreHIR pipeline (Refactor target)"
---
# Selfhost Compiler: Implement CoreHIR pipeline (Refactor target)

## Summary

The #529 100% selfhost transition ported the compiler using the `Legacy` pipeline baseline (`Lexer -> Parser -> Resolver -> TypeChecker -> MIR -> Wasm`). The documented `Refactor target` architecture (`Lex -> Parse -> ... -> Check+BuildCoreHIR -> LowerToMIR -> ...`) was not implemented during this initial bootstrap phase.

This issue tracks the implementation of the `src/compiler/corehir.ark` lowering pass and the subsequent restructuring of the compiler driver (`src/compiler/driver.ark` / `src/compiler/mir.ark`) to adopt the CoreHIR pipeline as the single source of truth for MIR lowering.

## Acceptance Criteria

- [x] Implement `src/compiler/corehir.ark` (AST to CoreHIR translation structures).
- [x] Implement CoreHIR to MIR lowering pass (replacing direct AST-to-MIR loops).
- [x] Wire the unified pipeline in `driver.ark`.
- [x] Verify all test fixtures pass utilizing the new CoreHIR intermediate path.

## Close Note (2026-05-17)

Implemented a selfhost CoreHIR facade in `src/compiler/corehir.ark` and routed
`src/compiler/driver.ark` through `typecheck -> corehir::build_program ->
corehir::lower_to_mir -> MIR`. Component/WIT lowering now uses the same CoreHIR
facade via `corehir::lower_to_mir_no_prune`.

Verification:

- `python scripts/manager.py selfhost fixpoint`: PASS
- `python scripts/manager.py verify quick`: PASS, 23/23
- `python scripts/manager.py selfhost fixture-parity`: PASS
- `python scripts/manager.py selfhost diag-parity`: PASS
- `python scripts/manager.py selfhost parity --mode --cli`: PASS
- `cargo check --workspace`: PASS
- `python scripts/manager.py verify fixtures`: PASS, fixture parity `PASS=307 FAIL=0 SKIP=95`

The fixture harness still reports pre-existing skips, but no fixture failures
were introduced by routing the driver through CoreHIR.

## Downstream Impact

- Unblocks #125 (`compile()` defaults to CoreHIR path).
