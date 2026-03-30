# v5 Phase 3: Backend + Wasm emission epic

**Status**: done
**Updated**: 2026-03-30
**ID**: 165
**Depends on**: 179, 180
**Track**: main
**Blocks v1 exit**: no

**Status note**: Parent issue for backend work after typed HIR exists. Includes MIR lowering and Wasm emission.

## Summary

v5 backend は 1 本の "emitter" ではなく、typed IR を MIR に落とす段階と、決定的な Wasm バイナリを出す段階に分かれる。fixpoint まで含めて見ても、この 2 段を分離して追跡したほうが queue が実態に近い。

## Acceptance

- [x] #179, #180 が完了している
- [x] HIR→MIR lowering と Wasm binary emission の責務が別 issue に分かれている
- [x] T1/T3 backend 差分と deterministic output requirements が child issue に反映されている

## References

- `issues/open/164-v5-phase2-resolver-typechecker.md`
- `issues/done/168-v5-ir-spec-doc.md`
- `crates/ark-mir/src/lower/`
- `crates/ark-wasm/src/emit/`
