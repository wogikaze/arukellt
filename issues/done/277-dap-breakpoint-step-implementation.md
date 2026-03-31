# setBreakpoints/continue/next/disconnect を実装する

**Status**: done
**Created**: 2026-03-30
**Updated**: 2026-03-30
**ID**: 277
**Depends on**: 276
**Track**: parallel
**Blocks v1 exit**: no

## Summary

276 で基本状態取得動詞を実装した後、ブレークポイント設定と実行制御（continue / step / disconnect）を実装して、実際にブレークして追えるデバッグ体験を完成させる。

## Acceptance

- [x] `setBreakpoints` でソース行にブレークポイントを設定でき、ヒットした時に停止できる
- [x] `configurationDone` リクエストを処理できる
- [x] `continue` でブレークポイントから実行を再開できる
- [x] `next`（ステップオーバー）でソース行単位のステップ実行ができる
- [x] `disconnect` でデバッグセッションを正常終了できる

## Scope

- `crates/ark-dap/src/lib.rs` に `setBreakpoints / configurationDone / continue / next / disconnect` ハンドラを追加
- runtime へのブレークポイント注入インタフェースの実装
- ブレークポイントヒット時の `stopped` イベント送出

## References

- `crates/ark-dap/src/lib.rs`
- `issues/open/276-dap-core-verbs-implementation.md`
- `issues/open/255-dap-end-to-end-workflow.md`
