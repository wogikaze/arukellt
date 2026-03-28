# v3: 標準ライブラリ整備

> **状態**: 未着手 — v2 完了後に着手可能

---

## 1. 版の目的

v2 までに確立した言語基盤の上で、モノモーフ関数群 (`Vec_new_i32`, `map_i32_i32` 等) を体系化された標準ライブラリに昇格させる。モジュール体系を導入し、命名規約を統一し、API 安定性ルールを策定する。v5 のセルフホストに必要なすべての stdlib 関数を v3 で揃える。

---

## 2. 到達目標

1. `std::string`, `std::collections`, `std::result`, `std::option`, `std::io`, `std::fmt`, `std::convert`, `std::test` モジュールが使用可能になる
2. 現行のモノモーフ名 (`Vec_new_i32`, `map_i32_i32` 等) から新 API への移行パスが `docs/migration/v2-to-v3.md` に記載されている
3. v5 セルフホストに必要な stdlib 関数 (文字列操作、Vec/HashMap、Result/Option、ファイル I/O、CLI 引数、プロセス終了制御) が全て実装・テスト済みである
4. Stable / Unstable / Deprecated の 3 段階 API 安定性マトリクスが確立している
5. 全 fixture tests (v2 時点の全件 + 新規追加分) が pass する

---

## 3. 対象範囲

| 対象 | 変更内容 |
|------|---------|
| `std/prelude.ark` | モジュール構造化、新 API 追加 |
| `std/manifest.toml` | 新 stdlib 関数の全件登録 |
| `crates/ark-stdlib/src/` | stdlib クレートの再構成 |
| `crates/ark-resolve/src/` | モジュール名前解決の実装 (`use std::string`) |
| `crates/ark-typecheck/src/` | ジェネリック API のモノモーフ化支援 |
| `crates/ark-wasm/src/emit/t3_wasm_gc.rs` | 新 stdlib ビルトインの emit サポート |
| `tests/fixtures/` | stdlib API を使う fixture の追加 (最低 30 件) |
| `docs/stdlib/reference.md` (新規または拡充) | 全 stdlib 関数のリファレンス |
| `docs/migration/v2-to-v3.md` (新規) | 旧 API → 新 API 移行ガイド |

---

## 4. 非対象範囲

- ネットワーク I/O: v3 では設計のみ、実装は v4 以降
- 正規表現: v3 では非対象。`contains`, `replace`, `split` で十分
- JSON/TOML パーサー: v3 では非対象 (stdlib 関数で実装可能になったら v4 で評価)
- 並行処理 (async/spawn): v5 (T5 WASI P3) スコープ
- GUI / グラフィクス: 非目標
- トレイト (`trait` キーワード): ADR-004 P3 としての評価は v3 完了後に判断。v3 では実装しない
- メソッド構文 (`.method()`): ADR-004 P4 として v4 以降
- `BTreeMap<K, V>`: v3 は HashMap のみ。BTreeMap は v4 で評価
- ネストジェネリクス (`Vec<Vec<T>>`): v3 の締めくくりで必要性を再評価し、解禁する場合は ADR-009 を記録

---

## 5. 主要設計課題

### 5.1 命名規約の移行

現行: `Vec_new_i32(n)`, `map_i32_i32(v, f)`, `filter_String(v, f)`  
目標: モジュール関数形式 (`std::collections::vec_new(n)`) または自由関数 (`vec_new(n)`)

**判断**: v3 では ADR-004 P3 (traits) と P4 (methods) が未確定のため、メソッド構文 (`v.map(f)`) への移行は v4 以降とする。v3 では自由関数 + 型推論によるモノモーフ化 (`vec_new(n)` → `Vec_new_i32` に展開) を採用する。旧名 (`Vec_new_i32`) は v3 で `#[deprecated]` にし、1 マイナー版後に除去する。

### 5.2 HashMap の実装

v3 で実装する HashMap はセルフホスト (v5) で多用される。実装要件:

```wat
(type $hashmap_i32_i32 (struct
  (field $keys   (mut (ref $arr_i32)))
  (field $values (mut (ref $arr_i32)))
  (field $count  (mut i32))
  (field $cap    (mut i32))
))
```

- 初期容量 16、負荷係数 0.75 で rehash (2× grow)
- オープンアドレッシング (linear probing) を採用 — linked list の GC alloc を避けるため
- `HashMap<String, i32>`, `HashMap<i32, i32>`, `HashMap<String, String>` のモノモーフ版を実装

### 5.3 ファイル I/O

WASI P2 の `wasi:filesystem/types` を使う。T3 での実装:
- `fs_read_file(path: String) -> Result<String, String>`
- `fs_write_file(path: String, content: String) -> Result<(), String>`
- `fs_exists(path: String) -> bool`

これは WASI P2 のみ対応 (T1 では非対応)。

### 5.4 モジュール名前解決

`use std::string` 宣言の解析は `ark-resolve` に追加する。モジュールパスの解決は `crates/ark-stdlib/src/modules.rs` で管理する。v3 では `std::*` の組み込みモジュールのみサポート。ユーザー定義モジュール (別ファイル分割) は v4 以降で評価。

### 5.5 API 安定性マトリクス

| 段階 | 定義 | 変更ルール |
|------|------|-----------|
| Stable | 公開 API で後方互換を保証 | 破壊的変更は Deprecated 経由のみ |
| Unstable | 実験的 API | 予告なし変更あり |
| Deprecated | 廃止予定 | 次マイナー版で除去 |

v3 で Stable にする最小セット: `string_concat`, `string_len`, `vec_new`, `vec_push`, `vec_len`, `vec_get`, `hashmap_new`, `hashmap_insert`, `hashmap_get`, `hashmap_contains_key`, `fs_read_file`, `fs_write_file`, `print`, `println`, `parse_i32`, `parse_i64`, `parse_f64`

---

## 6. 実装タスク

1. **`std/prelude.ark` のモジュール構造化** (`std/prelude.ark`, `std/manifest.toml`)  
   - 既存関数を `std::string`, `std::collections`, `std::result`, `std::option`, `std::io`, `std::fmt`, `std::convert`, `std::test` に分類。
   - `std/manifest.toml` にモジュール帰属を追記。
   - `scripts/check-stdlib-manifest.sh` でモジュール帰属の整合を検証。

2. **`ark-resolve` へのモジュール名前解決追加** (`crates/ark-resolve/src/`)  
   - `use std::string;` の構文解析と名前空間への展開。
   - `use std::collections::{vec_new, hashmap_new};` の destructure import。
   - モジュール解決テストを `crates/ark-resolve/tests/` に追加。

3. **HashMap の完全実装** (`std/prelude.ark`, `crates/ark-wasm/src/emit/t3_wasm_gc.rs`)  
   - `HashMap<i32, i32>`, `HashMap<String, i32>`, `HashMap<String, String>` のモノモーフ版。
   - `new`, `insert`, `get`, `contains_key`, `remove`, `len`, `keys`, `values` の実装。
   - GC struct 型定義 + open addressing rehash ロジック。
   - fixture: `tests/fixtures/collections/hashmap_rehash.ark` (30 件中必須)

4. **ファイル I/O の完全実装** (`std/prelude.ark`, `crates/ark-wasm/src/emit/t3_wasm_gc.rs`)  
   - `fs_read_file`, `fs_write_file`, `fs_exists`, `fs_mkdir`, `args` (コマンドライン引数取得), `exit` (終了コード制御)。
   - T3 (WASI P2) のみ対応。T1 では "not supported on T1" エラーを返す。

5. **`std::test` モジュールの実装**  
   - `assert`, `assert_eq`, `assert_ne`, `assert_true`, `assert_false` を `std::test` に移動。
   - パニック時のメッセージフォーマット改善 (`assert_eq: left=3, right=4`)。

6. **旧 API の Deprecated 化と移行ガイド**  
   - `Vec_new_i32` 等のモノモーフ名を `#[deprecated]` としてマーク (`std/manifest.toml`)。
   - `docs/migration/v2-to-v3.md` に旧名 → 新名の対応表を記載。
   - `verify-harness.sh` に deprecated API 使用の警告チェックを追加。

7. **stdlib API 安定性マトリクスの策定**  
   - `docs/stdlib/reference.md` に Stable/Unstable/Deprecated 表を作成。
   - 各関数に安定性ラベルを付与。
   - `std/manifest.toml` に `stability` フィールドを追加。

8. **ネストジェネリクス要否の評価**  
   - v3 完了時点で `Vec<Vec<T>>` が必要なユースケースを fixture ベースで調査。
   - 必要と判断した場合は ADR-009 を作成し v4 で実装。不要なら v5 まで禁止を維持。

9. **新規 fixture 追加** (最低 30 件)  
   - `tests/fixtures/stdlib/` に stdlib API ごとの fixture を追加。
   - HashMap rehash, ファイル I/O (T3 のみ), モジュール import, 安定性確認テストを含める。

---

## 7. 検証方法

```bash
# 全 fixture (v2 既存 + v3 新規)
cargo test -p arukellt --test harness -- --nocapture

# stdlib manifest 整合
scripts/check-stdlib-manifest.sh

# deprecated API 検出
scripts/verify-harness.sh  # deprecated check gate が含まれる

# ファイル I/O テスト (T3)
arukellt compile tests/fixtures/stdlib/fs_read_write.ark --target t3
wasmtime run --dir=. fs_read_write.wasm
```

---

## 8. 完了条件

| 条件 | 判定方法 |
|------|---------|
| `std::string`, `std::collections`, `std::result`, `std::option`, `std::io`, `std::fmt`, `std::convert`, `std::test` が使用可能 | fixture で `use std::string` が通る |
| HashMap (3 モノモーフ版) が全 CRUD 操作で正しく動作する | fixture pass |
| `fs_read_file`, `fs_write_file`, `args`, `exit` が T3 で動作する | fixture pass (T3) |
| 旧モノモーフ名 (`Vec_new_i32` 等) が `#[deprecated]` になっている | manifest.toml 確認 |
| `docs/migration/v2-to-v3.md` が存在し対応表が完備している | ファイル存在確認 |
| `docs/stdlib/reference.md` に Stable/Unstable/Deprecated 表が存在する | ファイル確認 |
| `scripts/verify-harness.sh` の全ゲートが通る (新規 gate 含む) | exit code 0 |
| v5 セルフホストに必要な stdlib 関数チェックリストが全件 Stable である | `std/manifest.toml` 確認 |

---

## 9. 次版 (v4) への受け渡し

v4 が開始できる前提条件:

1. v3 の全完了条件が達成されていること
2. API 安定性マトリクスが確立し、ベンチマーク対象 API が Stable になっていること (API が変わるとベンチマーク基準が無効化される)
3. ネストジェネリクス要否の判断 (ADR-009 作成 or 禁止継続の明記) が完了していること
4. v5 セルフホスト必要 stdlib 関数のチェックリストが全件揃っていること

**v3 → v4 に渡す成果物**:

| 成果物 | パス |
|--------|------|
| 完全な stdlib | `std/prelude.ark`, `std/manifest.toml` |
| HashMap GC 実装 | `t3_wasm_gc.rs` |
| API 安定性マトリクス | `docs/stdlib/reference.md` |
| モジュール名前解決 | `crates/ark-resolve/src/` |
| 移行ガイド | `docs/migration/v2-to-v3.md` |

---

## 10. この版で特に気をつけること

1. **セルフホスト必要 stdlib の先取り禁止**: セルフホスト (v5) 用に過剰な stdlib を v3 に入れない。必要な関数の「チェックリスト確認と実装」に留める。設計のみ行い実装を v4 に送る機能は明示すること。
2. **API 安定性ルールを決める前に実装しない**: Stable にする API の最小セットを決めてから実装を開始する。後で「Stable にしたが使われない」関数を増やさない。
3. **HashMap のモノモーフ数**: `HashMap<K, V>` のモノモーフ版は組み合わせ爆発する (K × V = 型数)。v3 では `<i32,i32>`, `<String,i32>`, `<String,String>` の 3 種のみ実装し、4 種目の追加は ADR 評価を経ること。
4. **`use std::string` の解決が既存コードを壊さないこと**: 現行の fixture は `use` 宣言なしで stdlib 関数を呼ぶ (prelude に全て入っている)。モジュール化後も prelude が維持され、既存 fixture が壊れないこと。移行期間中は `use std::string` と prelude 直接呼び出しの両方が動作すること。
5. **deprecated check の誤検知**: `manifest.toml` に `stability = "deprecated"` を付けた関数が fixture で使われている場合、verify-harness.sh が失敗する前に既存 fixture を移行済みにすること。
6. **ファイル I/O は T3 のみ**: `fs_read_file` 等を T1 で呼ぶと実行時エラーではなくコンパイルエラーにすること (ターゲット固有 API チェックを `ark-typecheck` または `ark-resolve` に追加)。
7. **`args()` の WASI P2 API**: コマンドライン引数は WASI P2 の `wasi:cli/environment.get-arguments` を使用。T1 (WASI P1) でも `args_get` で同等機能があるため、T1 対応を後から追加しやすい設計にすること。

---

## 11. この版で必ず残すドキュメント

| ドキュメント | パス | 内容 |
|------------|------|------|
| stdlib リファレンス | `docs/stdlib/reference.md` | 全関数 + 安定性ラベル + 型シグネチャ |
| v2→v3 移行ガイド | `docs/migration/v2-to-v3.md` | 旧 API → 新 API 対応表、`use` 宣言の追加方法 |
| 現状ドキュメント更新 | `docs/current-state.md` | v3 完了状態、stdlib モジュール一覧 |
| モジュールシステム設計 | `docs/language/type-system.md` | モジュール名前解決の仕様 |
| (必要な場合) ADR-009 | `docs/adr/ADR-009-nested-generics.md` | ネストジェネリクス解禁の判断 |

---

## 12. 未解決論点

1. **`From`/`Into` 相当のトレイト**: `std::convert` を体系化するには型変換トレイトが必要。v3 では traits なし (ADR-004) のため、個別変換関数 (`i32_to_string`, `string_to_i32` 等) で代替する。トレイト導入 (P3) は ADR-004 に従い v4 以降で評価。
2. **Set<T> の実装**: セルフホストには Set が必要か調査中。HashMap のキー集合で代替できる場合は v4 以降に送る。
3. **`Duration` 型**: `std::time` の `Duration` 型は struct で表現可能だが、演算子オーバーロード (ADR-004 P5) がないと使いにくい。v3 では `Duration { secs: i64, nanos: i32 }` を struct として定義し、演算は自由関数で提供する。
4. **v5 セルフホスト必要 stdlib の確定**: v3 完了時に「v5 セルフホストに必要な stdlib 関数チェックリスト」を作成し、欠落があれば v4 で補完する (仕様: `docs/process/selfhosting-stdlib-checklist.md` として作成)。
