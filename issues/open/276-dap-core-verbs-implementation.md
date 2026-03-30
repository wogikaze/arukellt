# DAP 基本動詞を実装する（launch/threads/stackTrace/scopes/variables）

**Status**: open
**Created**: 2026-03-30
**Updated**: 2026-03-30
**ID**: 276
**Depends on**: 255
**Track**: parallel
**Blocks v1 exit**: no

## Summary

`crates/ark-dap/src/lib.rs` は DAP transport を受ける最小 scaffold だが、日常デバッグに必要な基本動詞が未実装である。まず `threads / stackTrace / scopes / variables` のレスポンスを実装し、デバッガが「止まった後に状態を見る」ことを可能にする。

## Acceptance

- [ ] `threads` リクエストに対して現在実行中のスレッドリストを返せる
- [ ] `stackTrace` リクエストに対してコールスタックフレームを返せる
- [ ] `scopes` リクエストに対してスコープリストを返せる
- [ ] `variables` リクエストに対して変数名・型・値を返せる
- [ ] `launch` リクエストで `.ark` プログラムを起動できる

## Scope

- `crates/ark-dap/src/lib.rs` に `threads / stackTrace / scopes / variables / launch` ハンドラを追加
- runtime / MIR との接続インタフェースの設計
- 各レスポンスの DAP プロトコル準拠チェック（`debugpy` 等の spec を参照）

## References

- `crates/ark-dap/src/lib.rs`
- `issues/open/255-dap-end-to-end-workflow.md`
