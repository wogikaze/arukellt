---
Status: done
Created: 2026-04-15
Updated: 2026-04-22
Track: stdlib
Orchestration class: implementation-ready
Depends on: none
Closed: 2026-04-22
Commit: ebb57f4b
ID: 516
Orchestration upstream: —
Blocks v{N}: none
Source: stdlib modernization backlog requested 2026-04-15
Raw と Adapter の違い: "Raw は「stdlib **内**の表現をそのまま触る」、Adapter は「**外**とのプロトコルに合わせて変換する」。同一ファイル内でも両方が混ざりうる（例: I/O で `Vec<i32>`  writer を組み立てたあと intrinsic で出力）。"
---

# Stdlib: raw helper と推奨 facade の境界を再設計する
補足: 既存の無接頭辞名は、移行時に facade 側へ昇格するか `raw_` へ降格するかを family ごとに決める。`unchecked_` は **パフォーマンスまたは既知安全コンテキスト専用**として数を絞る。
### `std: ":wit` (`std/wit/mod.ark`)"
| `reader_read` / `reader_read_exact` | Raw | cursor とスライス前提の破壊的操作 | `std/io/mod.ark: "49`, `std/io/mod.ark:65` |"
| `stdin` / `stdout` / `stderr` | Adapter (+ Raw handle) | fd タグ付き `Vec<i32>` を返すホスト橋渡し | `std/io/mod.ark: "125`, `std/io/mod.ark:132`, `std/io/mod.ark:139` |"
| `print_bytes` | Adapter | バイト列を文字列化して intrinsic 出力 | `std/io/mod.ark: 154` |
| `read_stdin_line` | Facade | 戻りは意味 API（現状はプレースホルダー） | `std/io/mod.ark: 149` |
| `writer_write_str` | Facade 寄り（下層は Adapter 分岐） | `String` 契約；stdout/stderr では intrinsic | `std/io/mod.ark: 205` |
| `buf_reader_new` / `buf_writer_new` | Raw | バッファレイアウト（`buf_cap` 等）を公開 | `std/io/mod.ark: "276`, `std/io/mod.ark:319` |"
| `copy_bytes` | Facade（raw ハンドル上の合成） | 推奨ユースケース向けの合成操作 | `std/io/mod.ark: 392` |
| `HashMap_String_i32_insert` 等（関数本体の直呼び） | Raw | モノモーフィック生成コードへの直結（wrapper 内限定が理想） | 例: "`std/collections/hash_map.ark:21` |"
| `hashset_i32_new` / `hashset_i32_insert` | Facade | `HashSet<i32>` 型で意図が表に出る | `std/collections/hash_set.ark: "9`, `std/collections/hash_set.ark:14` |"
| `WitType`（enum） | Facade（型） | ユーザーが扱う意味のある分類 | `std/wit/mod.ark: 6` |
| `wit_type_id` / `wit_type_from_id` | Raw / interop | 整数 ID ワイヤ・イントロスペクション用 | `std/wit/mod.ark: "37`, `std/wit/mod.ark:56` |"
| `wit_type_name` | Facade | 表示・ログ用の安定した文字列 | `std/wit/mod.ark: 74` |
| `wit_type_bool` 等の定数コンストラクタ | Facade | 列挙値の読みやすい生成 | 例: "`std/wit/mod.ark:23` |"
# Stdlib: raw helper と推奨 facade の境界を再設計する

## Summary

stdlib には内部表現に近い helper と user-facing facade が混在しており、
どの API を推奨するかが分かりにくい。raw helper を module-internal または clearly-named low-level tier に寄せ、
推奨 surface は facade 側に集約する。

## Repo evidence

- `std/io/mod.ark` は `Vec<i32>` ベースの reader/writer internal format を公開 surface でも広く使う
- `std/collections/hash_map.ark` / `hash_set.ark` は monomorphic wrapper と low-level helper が混在する
- `std/wit/mod.ark` は interop helper と user-facing meaning helper が混ざりやすい

## Three-layer model (raw helper / facade / adapter)

この issue で揃える語彙。**いまの公開 API がどの層に置けるか**と、**移行後にどこへ寄せたいか**を分けて議論する。

| Layer | 定義 | 典型な責務 |
|-------|------|------------|
| **Raw helper** | 内部表現・レイアウト・モノモーフィック intrinsics をそのまま前提にする API。呼び出し側が不変条件（インデックス、タグ値、マスクなど）を理解していることが期待される。 | `Vec<i32>` の reader/writer レイアウト、`HashMap_*` / `hash::*` 直呼び、WIT の数値 ID 直渡し など。 |
| **Facade** | 意味のある型・操作として安定したユーザー向け surface。内部表現は隠蔽し、`Result` やドキュメント上の契約で振る舞いを約束する。 | 将来的な opaque handle / 高レベル型、`String` や `Option` ベースの操作、推奨エントリポイント。 |
| **Adapter** | ホスト・ランタイム・FFI など **stdlib の外側**との境界。外部の契約（intrinsic、将来の WASI）と stdlib 内表現を変換する。 | `__intrinsic_*` 呼び出し、stdin/stdout の fd タグ意味、コンポーネント ABI とのブリッジ。 |


## Naming policy (`raw_`, `unchecked_`, `internal_`)

Facade をデフォルトの推奨経路にし、低レベル API は名前で段をはっきりさせる方針案。

| Prefix / 規則 | 意図 | 例（方針レベル） |
|----------------|------|------------------|
| **`raw_`** | 内部レイアウトやワイヤ形式を公開する。呼び出し側がレイアウト仕様に従う。 | `raw_reader_from_bytes` のように、現状の `reader_from_bytes` 相当を明示的に格下げ。 |
| **`unchecked_`** | 通常の facade が行う境界チェック・不変条件検証を省略する。安全なラッパの直内側に置く。 | 既存の `get_unchecked` 利用パターンに揃えた公開名（必要な family のみ）。 |
| **`internal_`** | モジュール外からは依存しない想定。**公開されても**「安定 semver 対象外」または docs / manifest で非推奨と明示。 | 将来 `std::io::internal_reader_read` のように、他モジュールからのみ呼ぶ補助。理想は `pub` を外して同ファイル・クレート内に閉じる。 |


## Per-family API tiering (representative sampling)

**Proposed tier** は「この issue 完了後の目標ラベル」であり、現状の実装が既にそうなっているとは限らない。代表例のみ列挙する。

### `std::io` (`std/io/mod.ark`)

| Representative API | Proposed tier | Note | Source |
|--------------------|---------------|------|--------|
| `reader_from_bytes` | Raw → `raw_*` 相当へ | Reader の `[cursor, b0..]` レイアウトをそのまま構築 | `std/io/mod.ark:27` |
| `reader_read` / `reader_read_exact` | Raw | cursor とスライス前提の破壊的操作 | `std/io/mod.ark:49`, `std/io/mod.ark:65` |
| `stdin` / `stdout` / `stderr` | Adapter (+ Raw handle) | fd タグ付き `Vec<i32>` を返すホスト橋渡し | `std/io/mod.ark:125`, `std/io/mod.ark:132`, `std/io/mod.ark:139` |
| `print_bytes` | Adapter | バイト列を文字列化して intrinsic 出力 | `std/io/mod.ark:154` |
| `read_stdin_line` | Facade | 戻りは意味 API（現状はプレースホルダー） | `std/io/mod.ark:149` |
| `writer_write_str` | Facade 寄り（下層は Adapter 分岐） | `String` 契約；stdout/stderr では intrinsic | `std/io/mod.ark:205` |
| `buf_reader_new` / `buf_writer_new` | Raw | バッファレイアウト（`buf_cap` 等）を公開 | `std/io/mod.ark:276`, `std/io/mod.ark:319` |
| `copy_bytes` | Facade（raw ハンドル上の合成） | 推奨ユースケース向けの合成操作 | `std/io/mod.ark:392` |

### `std::collections` — `hash_map` / `hash_set`

#### `hash_map` (`std/collections/hash_map.ark`)

| Representative API | Proposed tier | Note | Source |
|--------------------|---------------|------|--------|
| `hashmap_str_i32_new` ほか `hashmap_*_*` 公開関数 | Facade | 意味のある `HashMap<K,V>` 操作；実体は intrinsic | 例: `std/collections/hash_map.ark:15`, `std/collections/hash_map.ark:20` |
| `HashMap_String_i32_insert` 等（関数本体の直呼び） | Raw | モノモーフィック生成コードへの直結（wrapper 内限定が理想） | 例: `std/collections/hash_map.ark:21` |

#### `hash_set` (`std/collections/hash_set.ark`)

| Representative API | Proposed tier | Note | Source |
|--------------------|---------------|------|--------|
| `hashset_i32_new` / `hashset_i32_insert` | Facade | `HashSet<i32>` 型で意図が表に出る | `std/collections/hash_set.ark:9`, `std/collections/hash_set.ark:14` |
| `hashset_str_new` ほか `hashset_str_*` | Raw（→ Facade へ） | 現状 `Vec<String>` キャリアで文字列集合を表す — 将来は `HashSet<String>` facade へ | 例: `std/collections/hash_set.ark:59`, `std/collections/hash_set.ark:64` |

### `std::wit` (`std/wit/mod.ark`)

| Representative API | Proposed tier | Note | Source |
|--------------------|---------------|------|--------|
| `WitType`（enum） | Facade（型） | ユーザーが扱う意味のある分類 | `std/wit/mod.ark:6` |
| `wit_type_id` / `wit_type_from_id` | Raw / interop | 整数 ID ワイヤ・イントロスペクション用 | `std/wit/mod.ark:37`, `std/wit/mod.ark:56` |
| `wit_type_name` | Facade | 表示・ログ用の安定した文字列 | `std/wit/mod.ark:74` |
| `wit_type_bool` 等の定数コンストラクタ | Facade | 列挙値の読みやすい生成 | 例: `std/wit/mod.ark:23` |

## Acceptance

- [x] raw helper / facade / adapter の 3 層分類が family ごとに作られる
- [x] low-level internal representation を直接公開している API が洗い出される
- [x] facade 優先の naming policy (`raw_`, `unchecked_`, `internal_` など) が定義される
- [x] 代表 family (`io`, `collections`, `wit`) の migration sketch が作られる

## Primary paths

- `std/io/mod.ark`
- `std/collections/hash_map.ark`
- `std/collections/hash_set.ark`
- `std/wit/mod.ark`
- `docs/stdlib/`

## References

- `issues/done/384-stdlib-api-admission-gate.md`