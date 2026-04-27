---
Status: done
Created: 2026-03-28
Updated: 2026-03-30
ID: 247
Track: docs
Depends on: none
Orchestration class: implementation-ready
Blocks v1 exit: no
---

# migration guide v2→v3: standard library module system completed
stdlib のモジュール整理（`std: ":core`, `std::text`, `std::collections` 等）、"
元ドキュメント: `docs/migration/v2-to-v3.md`（issue 化により移動済み）
- [x] `use std: ":*` モジュールパスが機能する"
- [x] `std: ":core`, `std::text`, `std::bytes`, `std::collections`, `std::io`, `std::env` モジュールが利用できる"
- prelude に含まれない stdlib 関数には明示的な `use std: ":*` import を追加"
# migration guide v2→v3: standard library module system completed

## Summary

v2→v3 の標準ライブラリモジュールシステム実装が完了したことの追跡 issue。
stdlib のモジュール整理（`std::core`, `std::text`, `std::collections` 等）、
API 命名規則統一、stability label、prelude 再エクスポート、生成リファレンスドキュメントは完了している。
Migration Checklist の項目はユーザーコード側の対応であり、言語実装側の完了をもってこの issue は done とする。

元ドキュメント: `docs/migration/v2-to-v3.md`（issue 化により移動済み）

## Acceptance

- [x] `use std::*` モジュールパスが機能する
- [x] prelude が最頻出名前を再エクスポートしている
- [x] 旧 monomorphic API（`Vec_new_i32` 等）が compiler 警告で deprecated として検出される
- [x] `std::core`, `std::text`, `std::bytes`, `std::collections`, `std::io`, `std::env` モジュールが利用できる
- [x] stability label が stdlib リファレンスに反映されている

## User Migration Checklist

以下はユーザーコード側の対応事項（言語実装の完了条件ではない）：

- `docs/stdlib/prelude-migration.md` でprelude から削除された名前を確認
- prelude に含まれない stdlib 関数には明示的な `use std::*` import を追加
- compiler 警告で deprecated と指摘された関数名を置き換え
- （任意）ライブラリコードを書く場合は `docs/stdlib/stability-policy.md` のラベルを採用
- （任意）stdlib に変更を加えた場合は stdlib リファレンスを再生成

## References

- `std/manifest.toml`
- `docs/current-state.md`