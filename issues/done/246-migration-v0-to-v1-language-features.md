# migration guide v0→v1: language features completed, user migration tracking

**Status**: done
**Created**: 2026-03-28
**Updated**: 2026-03-30
**ID**: 246
**Depends on**: none
**Track**: docs
**Blocks v1 exit**: no

## Summary

v0→v1 の言語機能実装が完了したことの追跡 issue。
言語実装（`parse_i64`/`parse_f64` の Result 型化、予約キーワード追加、trait/impl/operator overloading/pattern matching 拡張）はすべて完了している。
Migration Checklist の項目はユーザーコード側の対応であり、言語実装側の完了をもってこの issue は done とする。

元ドキュメント: `docs/migration/v0-to-v1.md`（issue 化により移動済み）

## Acceptance

- [x] `parse_i64()` / `parse_f64()` が `Result<T, String>` を返す
- [x] `trait`, `impl`, `for`, `in` が予約キーワードとして機能する
- [x] method call syntax (`obj.method()`) が動作する
- [x] operator overloading (`add`, `sub`, `eq` 等) が動作する
- [x] guard / or-pattern / struct pattern / tuple pattern が動作する
- [x] struct field update (`{ ..base }`) が動作する
- [x] nested generics および user-defined generic structs が動作する

## User Migration Checklist

以下はユーザーコード側の対応事項（言語実装の完了条件ではない）：

- すべての `parse_i64()` / `parse_f64()` 呼び出し箇所で `Result` を処理するよう更新
- `trait`, `impl`, `for`, `in` という識別子名を使っていた場合はリネーム
- （任意）関数呼び出しスタイルからメソッド構文に移行
- （任意）カスタム型に operator overloading を追加
- （任意）match 式で guard / or-pattern / struct pattern を活用

## References

- `docs/migration/` (v0-to-v1.md は本 issue 化により移動)
- `docs/current-state.md`
