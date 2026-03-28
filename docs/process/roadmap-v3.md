# v3: 標準ライブラリ整備

> **状態**: **進行中** — issues #039–#059 (21 件 open) で追跡中  
> 設計書: `docs/stdlib/std.md` / モジュール仕様: `docs/stdlib/modules/`

---

## 1. 版の目的

v2 までに確立した言語基盤の上で、モノモーフ関数群 (`Vec_new_i32`, `map_i32_i32` 等) を体系化された標準ライブラリに昇格させる。モジュール体系を導入し、命名規約を統一し、API 安定性ルールを策定する。v5 のセルフホストに必要なすべての stdlib 関数を v3 で揃える。

---

## 2. 到達目標

1. `use std::*` によるモジュール import が動作する (`std::core`, `std::text`, `std::bytes`, `std::collections`, `std::seq`, `std::io`, `std::time`, `std::random`, `std::process`, `std::wasm`, `std::wit`, `std::test`)
2. 現行のモノモーフ名 (`Vec_new_i32`, `map_i32_i32` 等) から新 API への移行パスが `docs/migration/v2-to-v3.md` に記載されている
3. v5 セルフホストに必要な stdlib 関数 (文字列操作・Vec/HashMap/HashSet/Deque・Result/Option・ファイル I/O・CLI 引数・プロセス終了制御) が全て実装・テスト済みである
4. Stable / Experimental / Deprecated の 3 段階 API 安定性ラベルが全 public 関数に付与されている
5. 全 fixture tests (v2 時点の全件 + v3 新規 40 件以上) が pass する
6. Scalar 型完全化 (`u8`, `u16`, `u32`, `u64`, `i8`, `i16`, `f32`) が言語基盤に追加されている
7. `Seq<T>` 遅延シーケンスによる `map/filter/fold/zip/collect` が動作する
8. `docs/stdlib/std.md` の「付録 B 最小 Stable セット」が全て Stable として実装済みである

---

## 3. 対象範囲

| 対象 | 変更内容 | issue |
|------|---------|-------|
| モジュールシステム (`use std::*`) | `ark-resolve` にモジュール名前解決追加 | #039 |
| Scalar 型完全化 | `u8/u16/u32/u64/i8/i16/f32` を言語・stdlib に追加 | #040 |
| `std::core` | Error 型、Ordering、Range、cmp、math、convert、hash | #041 |
| `std::text` | String API 拡張、StringBuilder、fmt、Rope | #042 / #047 |
| `std::bytes` | Bytes、ByteBuf、ByteView、ByteCursor、endian、hex、base64、leb128 | #043 |
| `std::collections::hash` | HashMap\<K,V\> 汎用化と HashSet\<T\> | #044 |
| `std::collections` (linear) | Deque、PriorityQueue | #045 |
| `std::collections` (ordered) | BTreeMap、BTreeSet、IndexMap、IndexSet、BitSet | #046 |
| `std::collections` (compiler) | Arena、SlotMap、Interner (Experimental) | #047 |
| `std::seq` | Seq\<T\> 遅延シーケンス + sort/search/group | #048 |
| `std::path` + `std::fs` | パス操作、ファイル I/O (WASI P2) | #049 |
| `std::io` | Reader、Writer、stdin/stdout/stderr、buffered I/O | #050 |
| `std::time` + `std::random` | Instant、Duration、seeded RNG | #051 |
| `std::process` + `std::env` + `std::cli` | args、exit、env vars | #052 |
| `std::wasm` | Wasm バイナリ型・opcode・module builder (Experimental) | #053 |
| `std::wit` + `std::component` | WIT 型、resource handle、canonical ABI (Experimental) | #054 |
| `std::json` + `std::toml` + `std::csv` | データ形式パーサ (Experimental) | #055 |
| `std::test` | assert_ok/err/some/none + snapshot + bench-lite | #056 |
| Prelude 再構成 | Prelude 縮小 + 旧 API deprecated 化 | #057 |
| API 安定性ラベル + ドキュメント | stability labels + `docs/stdlib/reference.md` 完成 | #058 |
| v3 fixture 統合 | 40 件以上 fixture + verify-harness.sh Check 18–20 | #059 |

---

## 4. 非対象範囲

- ネットワーク I/O: v4 以降
- 正規表現: v3 では非対象。`contains`, `replace`, `split` で十分
- 並行処理 (async/spawn): v5 (T5 WASI P3) スコープ
- GUI / グラフィクス: 非目標
- トレイト (`trait` キーワード): ADR-004 P3 としての評価は v3 完了後に判断。v3 では実装しない
- メソッド構文 (`.method()`): ADR-004 P4 として v4 以降
- ネストジェネリクス (`Vec<Vec<T>>`): v3 完了時点で必要性を再評価し、解禁する場合は ADR-009 を記録
- WIT first (WIT ファイル → Arukellt スタブ生成): #054 は Arukellt first のみ。WIT first は v4 で評価
- async Component (WASI P3): T5 スコープ。v3 では `future<T>`, `stream<T>` を非対応エラーとする
- `BTreeMap` 以外の自己平衡木 (AVL, Red-Black): IndexMap/BTreeMap で十分な範囲に限定
- JSON/TOML/CSV の streaming decode: #055 は Experimental として基本 parse/stringify のみ

---

## 5. 主要設計課題

### 5.1 命名規約の移行

現行: `Vec_new_i32(n)`, `map_i32_i32(v, f)`, `filter_String(v, f)`  
目標: モジュール関数形式 `vec::new<i32>(n)`, `seq::map(v, f)` (std.md §13.3 命名規約)

**判断**: v3 では自由関数 + 明示的モジュール prefix (`vec::new`, `string::split`) を採用。メソッド構文 (`v.map(f)`) は ADR-004 P4 として v4 以降。旧名 (`Vec_new_i32`) は v3 で `#[deprecated]` にし、#057 (Prelude 再構成) で移行を完了させる。移行表は `docs/stdlib/std.md §14.1`。

### 5.2 std::text と String/Bytes の分離

std.md §4 原則 2: **text と binary を分離する**。`String` は UTF-8 保証、`Bytes` は UTF-8 保証なしの raw binary。

- `String` → `(ref null (array (mut i8)))` に UTF-8 constraint (バリデーション via #042)
- `Bytes` → `(ref null (array (mut i8)))` と同じ GC 表現だが型が別 (コンパイラ区別)
- `ByteView` → zero-copy read-only slice (Experimental、#043)
- `Rope` → `std::text::rope` namespace (#047) — `std::collections` ではない

### 5.3 コレクション設計の全体像

std.md §7 に基づく採用判断 (v3 スコープ):

| 構造 | issue | 安定性 |
|------|-------|-------|
| Vec\<T\> | 既存 + #039 で汎用化 | Stable |
| HashMap\<K,V\> | #044 | Stable |
| HashSet\<T\> | #044 | Stable |
| Deque\<T\> | #045 | Stable |
| PriorityQueue\<T\> | #045 | Experimental |
| BTreeMap\<K,V\> | #046 | Experimental |
| BTreeSet\<T\> | #046 | Experimental |
| IndexMap\<K,V\> | #046 | Experimental |
| BitSet | #046 | Experimental |
| Arena\<T\> | #047 | Experimental |
| SlotMap\<V\> | #047 | Experimental |
| Interner\<T\> | #047 | Experimental |

### 5.4 モジュール名前解決

`use std::text::string` の解析は `ark-resolve` に追加する (#039)。v3 では `std::*` 組み込みモジュールのみサポート。ユーザー定義モジュール (別ファイル分割) は v4 以降。

**正準 import 例** (std.md §5.1):
```ark
use std::text::string
use std::bytes
use std::collections::{vec, hash_map}
use std::seq
```

### 5.5 API 安定性ラベル (3 段階)

std.md §12.1 に準拠:

| 段階 | 定義 | 変更ルール |
|------|------|-----------|
| Stable | 公開 API で後方互換を保証 | 破壊的変更は Deprecated 経由のみ |
| Experimental | 実験的 API | 予告なし変更あり |
| Deprecated | 廃止予定 | 次マイナー版で除去 |

v3 で Stable にする最小セット: `docs/stdlib/std.md` 付録 B を参照。

---

## 6. 実装タスク (issues #039–#059)

依存順の大まかな実行順序 (詳細は `issues/open/dependency-graph.md`):

1. **#039** — モジュールシステム基盤 (`use std::*` import インフラ) ← 全 v3 issues の先行依存
2. **#040** — Scalar 型完全化 (`u8/u16/u32/u64/i8/i16/f32`)
3. **#041** — `std::core` (Error 型、Ordering、Range、cmp、math、convert、hash)
4. **#043** — `std::bytes` (Bytes、ByteBuf、ByteView、ByteCursor、endian、hex、base64、leb128)
5. **#051** — `std::time` + `std::random`
6. **#042** — `std::text` (String API 拡張、StringBuilder、fmt)
7. **#044** — `std::collections::hash` (HashMap\<K,V\>、HashSet\<T\>)
8. **#045** — `std::collections` linear (Deque、PriorityQueue)
9. **#046** — `std::collections` ordered (BTreeMap、BTreeSet、IndexMap、IndexSet、BitSet)
10. **#047** — `std::collections` compiler (Arena、SlotMap、Interner / `std::text::rope`)
11. **#048** — `std::seq` (Seq\<T\> 遅延シーケンス + アルゴリズム)
12. **#056** — `std::test` (assert_ok/err/some/none + snapshot + bench-lite)
13. **#050** — `std::io` (Reader、Writer、stdin/stdout/stderr、buffered I/O)
14. **#049** — `std::path` + `std::fs` (パス操作、ファイル I/O)
15. **#053** — `std::wasm` (Wasm バイナリ型・opcode・module builder) [Experimental]
16. **#052** — `std::process` + `std::env` + `std::cli`
17. **#055** — `std::json` + `std::toml` + `std::csv` [Experimental]
18. **#054** — `std::wit` + `std::component` [Experimental]
19. **#057** — Prelude 再構成 + 旧 API deprecated 化 + 移行ガイド
20. **#058** — API 安定性ラベル確定 + `docs/stdlib/reference.md` 完成
21. **#059** — v3 fixture 統合 + verify-harness.sh Check 18–20

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
| `use std::text::string` 等のモジュール import が全 v3 モジュールで動作する | fixture で `use std::*` が通る (#039) |
| Scalar 型 `u8/u16/u32/u64/i8/i16/f32` が言語・stdlib で使用可能 | fixture pass (#040) |
| `std::core` の Error 型・Ordering・Range・math が動作する | fixture pass (#041) |
| `std::bytes` の Bytes/ByteBuf/ByteView/endian/hex/base64/leb128 が動作する | fixture 8 件以上 pass (#043) |
| HashMap\<K,V\>・HashSet\<T\> が汎用型で動作する | fixture pass (#044) |
| Deque・PriorityQueue が動作する | fixture pass (#045) |
| `Seq<T>` の map/filter/fold/zip/collect が動作する | fixture pass (#048) |
| `std::path` + `std::fs` の read/write/exists が T3 で動作する | fixture pass, T3 のみ (#049) |
| `std::process::args()` と `exit()` が動作する | fixture pass (#052) |
| 旧モノモーフ名 (`Vec_new_i32` 等) が Deprecated になっている | manifest.toml + verify-harness.sh Check 19 (#057) |
| `docs/migration/v2-to-v3.md` が存在し対応表が完備している | ファイル確認 (#057) |
| `docs/stdlib/reference.md` に全 public 関数の安定性ラベルが付与されている | ファイル確認 (#058) |
| `docs/stdlib/std.md` 付録 B の最小 Stable セットが全件 Stable として実装済み | reference.md 確認 (#058) |
| `scripts/verify-harness.sh` の全ゲートが通る (Check 18–20 含む) | exit code 0 (#059) |
| v3 fixture 40 件以上が pass する | cargo test harness (#059) |

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

1. **`From`/`Into` 相当のトレイト**: v3 では traits なし (ADR-004)。個別変換関数 (`i32_to_string` → `string::from_i32`) で代替。トレイト導入は v4 以降。
2. **ネストジェネリクス (`Vec<Vec<T>>`)**: v3 完了時点でユースケースを fixture ベースで調査。必要ならば ADR-009 を作成し v4 で実装、不要ならば v5 まで禁止を維持。
3. **`Duration` 型の演算**: 演算子オーバーロード (ADR-004 P5) がないため、v3 では `duration_add`, `duration_sub` 等の自由関数で提供。
4. **v5 セルフホスト必要 stdlib の確定**: v3 完了時に「v5 セルフホストに必要な stdlib 関数チェックリスト」を `docs/process/selfhosting-stdlib-checklist.md` として作成。欠落があれば v4 で補完。
5. **`std::json` の Stable 昇格タイミング**: #055 は Experimental。セルフホスト (v5) でパーサが JSON を出力するユースケースが判明した時点で Stable 候補として再評価する。
