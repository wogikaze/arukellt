# T1/T3 debug 対象範囲を定義し canonical path で end-to-end 確認する

**Status**: open
**Created**: 2026-03-30
**Updated**: 2026-03-30
**ID**: 279
**Depends on**: 277, 278
**Track**: parallel
**Blocks v1 exit**: no

## Summary

どの target・emit 種別がデバッグ対象かが明記されていないため、「DAP は実装されているが実際にどの環境で使えるか分からない」状態になりやすい。T1/T3 の debug 対象範囲を定義し、canonical path で end-to-end が動くことを確認する。

## Acceptance

- [ ] `docs/debug-support.md`（または `docs/current-state.md` のデバッグセクション）に T1/T3 それぞれの debug 対象範囲が記載されている
- [ ] 少なくとも 1 つの canonical target で `.ark` ファイルにブレークポイントを置いて実際に停止できる
- [ ] どの target が "supported" で どれが "best-effort" かが明記されている
- [ ] compiler の source location 情報と DAP の line/column が正しく対応している

## Scope

- `docs/debug-support.md` の作成
- T1/T3 のソース位置情報（location/source map）と DAP フレームの対応確認
- canonical target での手動 end-to-end 確認と結果の記録

## References

- `crates/ark-dap/src/lib.rs`
- `docs/current-state.md`
- `issues/open/277-dap-breakpoint-step-implementation.md`
- `issues/open/278-vscode-debug-contribution.md`
- `issues/open/255-dap-end-to-end-workflow.md`
