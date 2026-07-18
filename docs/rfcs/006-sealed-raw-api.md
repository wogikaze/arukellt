# RFC-006: Sealed raw API for Vec/String representation

ステータス: ACCEPTED  
関連 ADR: [ADR-042](../adr/ADR-042-intrinsic-layer-separation.md)（D4）  
関連 issue: [#817](../../issues/done/817-sealed-raw-api-module.md)  
関連 plan: [intrinsic-layer-separation.md](../plans/intrinsic-layer-separation.md)  
日付: 2026-07-16

本 RFC は ADR-042 D4 の未決事項（モジュール名と公開面）を閉じる。
実装 owner は #817。意味論の stdlib 移行は #822。

---

## 要約

Vec/String の内部表現へ触れる入口を **`core::raw`** に統一する。
このモジュールは stdlib 専用の sealed API であり、ユーザーコードからの
import / パス参照は compile-time に拒否する。GC と linear-memory の表現差は
`core::raw` 実装層に閉じ込め、`intrinsic_*_gc.ark` / `intrinsic_*_lm.ark`
の二重ファイルを #817 で吸収・削除する。

---

## 動機 / 問題

現状 Vec/String 操作は emitter 内の GC/LM 分岐と prelude/`std` の
`__intrinsic_*` 直呼びに散在している。ADR-042 D4 は sealed raw API を要求するが、
モジュール名が未決のため #822 が着手できない。

---

## 提案

### D1: モジュール名は `core::raw`

採択: **`core::raw`**

| 候補 | 結果 |
|------|------|
| `core::raw` | **採択**。表現層であることが名前から分かる |
| `core::rt` | 却下。runtime ABI（#819）と混同しやすい |
| `core::intrinsics` | 却下。emitter 時代の語感が残り、正規入口に見える |

パス: `std/core/raw.ark`（ロード名 `core::raw` / `std::core::raw` は
既存 stdlib パス規約に合わせる。公開資格子は `core::raw`）。

### D2: 最小 surface

汎用 array buffer（Vec の裏）:

- `raw_array_new<T>(cap: i32) -> RawArray<T>`
- `raw_array_len<T>(a: RawArray<T>) -> i32`
- `raw_array_get_unchecked<T>(a: RawArray<T>, i: i32) -> T`
- `raw_array_set_unchecked<T>(a: RawArray<T>, i: i32, v: T)`
- `raw_array_grow<T>(a: RawArray<T>, new_cap: i32) -> RawArray<T>`

String ストレージ:

- `raw_string_new() -> RawString`
- `raw_string_from_bytes(bytes: RawArray<u8>) -> RawString`（または等価）
- `raw_string_len_bytes(s: RawString) -> i32`
- `raw_string_byte_at_unchecked(s: RawString, i: i32) -> u8`
- `raw_string_clone(s: RawString) -> RawString`

型名 `RawArray` / `RawString` は sealed モジュール内の opaque typedef とし、
ユーザー型としては公開しない。初期実装では既存コンパイラ表現
（GC array / LM buffer）への typealias + CoreOp / target intrinsic 委譲でよい。

### D3: 可視性（sealed）

1. `core::raw` は `visibility = internal` / stdlib-only。
2. ユーザークレートからの `use core::raw` / `use std::core::raw` および
   修飾呼び出しは compile-time error（診断は actionable にする）。
3. `std::` / `core::` 配下のコンパイラ同梱モジュールのみが import できる。
4. ユーザー可達 free function として再エクスポートしない（ADR-044/046）。

### D4: API 形

sealed 入口はモジュール関数でよいが、**ユーザー可達 free function にしない**。
`Vec` / `String` の公開 API は method / associated を正規形とする（#718）。
`core::raw` は stdlib 実装の内部依存のみ。

### D5: GC / LM

表現差は `core::raw` の実装（およびそれが呼ぶ target intrinsic）に閉じる。
`src/compiler/wasm/intrinsic_*_gc.ark` と `intrinsic_*_lm.ark` の二重ファイルは、
raw 層へ移した操作から順に削除する。残差は #817 完了時にゼロを目指す。

### D6: #822 との境界

| #817 | #822 |
|------|------|
| 表現 primitive（new/len/get/set/grow、string bytes） | split/join/replace/HashMap 等の意味論 |
| sealed モジュールと可視性 | Ark stdlib body と differential |
| dual intrinsic 削除 | `legacy_emitter` からの本番 lowering 置換 |

---

## 移行手順（#817）

1. 本 RFC を ACCEPTED にする。
2. `std/core/raw.ark`（および必要なら型定義）を追加し manifest に internal 登録する。
3. loader / resolver に sealed import 検査を入れる。
4. `std/collections/vec.ark` / String 実装を `core::raw` 経由に切り替える。
5. GC/LM 二重ファイルを raw 実装へ吸収して削除する。
6. GC/LM differential と `verify quick` を通す。

---

## 代替案と却下理由

| 案 | 却下理由 |
|----|----------|
| ユーザーにも `unsafe` raw を公開 | ADR-042 D4 に反する |
| raw API を prelude に置く | prelude 復帰（#816）と責務が混ざる |
| 意味論（split 等）も raw に含める | #822 の責務。raw が肥大化する |

---

## 関連

- ADR-042 D4
- [#817](../../issues/done/817-sealed-raw-api-module.md)
- [#822](../../issues/open/822-representation-dependent-stdlib-migration.md)
- [#816](../../issues/done/816-prelude-compilation-restoration.md)
