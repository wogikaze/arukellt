# 160: セルフホスト必要 stdlib チェックリストの確認

**Version**: v5 (selfhost prep)
**Priority**: P0 — v5 着手の前提条件
**Depends on**: なし

## 概要

セルフホスト実装に必要な stdlib 機能を洗い出し、不足分を特定する。`docs/process/selfhosting-stdlib-checklist.md` を作成し、全件の実装状況を確認する。

## 必要な stdlib 機能

コンパイラ実装に必要:
- String: char_at, substring, contains, starts_with, ends_with, split, trim, len, concat, i32_to_string, i64_to_string, f64_to_string
- Vec: push, pop, get, set, len, contains, remove, map, filter
- HashMap: new, insert, get, contains_key, remove, keys, values, len
- Option: Some, None, unwrap, is_some, is_none, map
- Result: Ok, Err, unwrap, is_ok, is_err, map
- I/O: fs_read_file, print, println, eprintln, exit, args
- 数値: i32/i64/f64 の算術、ビット演算、比較

## タスク

1. 上記リストと std/manifest.toml を突合し、未実装項目を列挙
2. 未実装項目ごとに実装難易度と優先度を記載
3. チェックリストドキュメントを `docs/process/selfhosting-stdlib-checklist.md` に出力

## 完了条件

- チェックリストが作成され、全項目に ✅ (実装済み) / ❌ (未実装) のマーク
- 未実装の必須項目がゼロ、または別 issue で追跡されている
