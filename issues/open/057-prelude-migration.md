# Prelude 再構成と API 移行

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-03-28
**ID**: 057
**Depends on**: 039, 041, 042, 044, 048, 049, 052
**Track**: stdlib
**Blocks v3 exit**: yes

## Summary

現在の大きな prelude を「tiny prelude, large explicit stdlib」に再構成する。
旧モノモーフ名 (`Vec_new_i32`, `map_i32_i32`, `fs_read_file` 等) を deprecated 化し、
新 module-based API への移行パスを提供する。
migration guide の作成と deprecated warning の実装を含む。

## 背景

現在の prelude.ark は 70+ 関数を export しており、名前空間が過密。
std.md §14 は「新 API 追加 → 旧 API deprecated → 旧 API 除去」の三段階移行を明記。
v3 では第一段階と第二段階を実施し、旧 API 除去は v4 に送る。

## 受け入れ条件

### 新 prelude (tiny)

```ark
// 自動 import される最小セット
Option, Result, String, Bytes, Vec
Some, None, Ok, Err
panic, assert, println, eprintln
```

### 旧 API deprecated 化

以下を deprecated warning 付きで維持:

| 旧 API | 新 API (移行先) |
|---|---|
| `Vec_new_i32()` | `use std::collections::vec; vec::new<i32>()` |
| `Vec_new_i64()` | `vec::new<i64>()` |
| `Vec_new_f64()` | `vec::new<f64>()` |
| `Vec_new_String()` | `vec::new<String>()` |
| `map_i32_i32(v, f)` | `use std::seq; seq::map(seq::from_vec(v), f)` |
| `filter_i32(v, f)` | `seq::filter(seq::from_vec(v), f)` |
| `fold_i32_i32(v, init, f)` | `seq::fold(seq::from_vec(v), init, f)` |
| `sort_i32(v)` | `use std::seq::algo; algo::sort(v)` |
| `concat(a, b)` | `use std::text::string; string::concat(a, b)` |
| `split(s, sep)` | `string::split(s, sep)` |
| `fs_read_file(path)` | `use std::fs; fs::read_to_string(path::from_string(path))` |
| `HashMap_i32_i32_new()` | `use std::collections::hash_map; hash_map::new<i32, i32>()` |
| `random_i32()` | `use std::random; random::random_i32_range(0, 2147483647)` |

### Deprecated warning

コンパイル時に以下を出力:

```text
W0100: `Vec_new_i32` is deprecated. Use `std::collections::vec::new<i32>()` instead.
```

## 実装タスク

1. `std/prelude.ark` を最小化: 新 prelude に含める関数のみ残す
2. `std/prelude_compat.ark` (or 既存 prelude に inline): 旧関数を deprecated wrapper として維持
3. `ark-diagnostics`: W0100 (deprecated function) 警告コードを追加
4. `ark-resolve`: deprecated 関数呼び出し時に W0100 を emit
5. 既存 fixture を新 API に段階的に移行 (全部は不要 — 主要 10 個を移行)
6. migration guide 文書の作成

## 検証方法

- fixture: `stdlib_migration/old_api_warning.ark` (diag — W0100 が出ることを確認)
- fixture: `stdlib_migration/new_api_basic.ark` (新 API で動作することを確認)
- 全既存 fixture が pass すること (deprecated warning は出ても pass)

## 完了条件

- prelude の自動 import シンボル数が 15 個以下
- 旧 API 呼び出しで W0100 warning が出る
- 新 module-based API で全旧機能が利用可能
- migration guide が作成されている
- 全既存 fixture pass

## 注意点

1. 既存ユーザーコードを即座に壊さない — deprecated warning は warning のみ、error にしない
2. fixture の大量書き換えは不要 — 旧 API の deprecated wrapper が動作すれば pass
3. prelude 縮小は二段階: v3 で warning、v4 で旧 API 除去

## ドキュメント

- `docs/migration/v2-to-v3.md`: 旧 API → 新 API の完全写像表、移行手順
- `docs/stdlib/prelude.md`: 新 prelude の内容と設計理由

## 未解決論点

1. deprecated warning を出すタイミング: パース時 vs 型検査時 vs emit 時
2. 旧 API の除去を v4 で一括にするか、段階的にするか
3. ユーザーが `#[allow(deprecated)]` 相当でwarning を抑制できるようにするか
