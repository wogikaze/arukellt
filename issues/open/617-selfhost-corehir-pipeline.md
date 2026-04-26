# Selfhost Compiler: Implement CoreHIR pipeline (Refactor target)

**Status**: open
**Created**: 2026-04-25
**Updated**: 2026-04-25
**ID**: 617
**Depends on**: 529
**Track**: pipeline-refactor
**Orchestration class**: blocked-by-upstream
**Orchestration upstream**: #529
**Blocks v4 exit**: yes

## Summary

The #529 100% selfhost transition ported the compiler using the `Legacy` pipeline baseline (`Lexer -> Parser -> Resolver -> TypeChecker -> MIR -> Wasm`). The documented `Refactor target` architecture (`Lex -> Parse -> ... -> Check+BuildCoreHIR -> LowerToMIR -> ...`) was not implemented during this initial bootstrap phase.

This issue tracks the implementation of the `src/compiler/corehir.ark` lowering pass and the subsequent restructuring of the compiler driver (`src/compiler/driver.ark` / `src/compiler/mir.ark`) to adopt the CoreHIR pipeline as the single source of truth for MIR lowering.

## Acceptance Criteria

- [ ] Implement `src/compiler/corehir.ark` (AST to CoreHIR translation structures).
- [ ] Implement CoreHIR to MIR lowering pass (replacing direct AST-to-MIR loops).
- [ ] Wire the unified pipeline in `driver.ark`.
- [ ]- Verify all test fixtures pass utilizing the new CoreHIR intermediate path.

## Downstream Impact
- Unblocks #125 (`compile()` defaults to CoreHIR path).
