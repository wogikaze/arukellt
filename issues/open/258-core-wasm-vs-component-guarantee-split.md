# core Wasm と component 出力の保証レベルを分離する

**Status**: open
**Created**: 2026-03-30
**Updated**: 2026-03-30
**ID**: 258
**Depends on**: 257
**Track**: main
**Blocks v1 exit**: yes

## Summary

`--emit component` は `wasm32-wasi-p2` 上で利用可能とされているが、`wasm-tools` と adapter バイナリに依存するため、core Wasm 出力と component 出力の保証レベルは同列ではない。この区別が CI と docs の両方で曖昧になっている。

## Acceptance

- [ ] `docs/target-contract.md` で `emit-core` と `emit-component` が別行として定義されている
- [ ] CI で core Wasm 検証と component 検証が独立した step として実行される
- [ ] `wasm-tools` / adapter 依存の有無が target contract に明記されている
- [ ] component 出力が optional smoke tier であることが CI のジョブ名・docs の両方で明確になっている

## Scope

- `docs/target-contract.md` の emit 行を core / component に分割
- CI の component 出力 step に `wasm-tools` 依存チェックを追加
- `scripts/verify-harness.sh` の `--component` フラグと CI ジョブの対応を整理

## References

- `scripts/verify-harness.sh`
- `issues/open/257-target-contract-table.md`
- `issues/open/251-target-matrix-execution-contract.md`
