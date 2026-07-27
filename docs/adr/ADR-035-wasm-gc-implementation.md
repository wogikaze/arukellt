# ADR-035: Wasm GC 内部レイアウト方針

ステータス: **ACCEPTED** — `wasm32-gc` の compiler-private GC 表現と型 ownership を固定する

提案日: 2026-06-17  
決定日: 2026-07-18

---

## 文脈

ADR-002 / ADR-006 / ADR-007 / ADR-013 / ADR-040 は次を既に決定している。

- 言語意味論は Wasm GC を前提とする。
- primary ターゲットは `wasm32-gc` で、`wasm32` は同じ意味論の linear lowering とする。
- GC layout は compiler-private であり、stable ABI ではない。
- 公開相互運用は WIT / Canonical ABI を正本とする。
- MIR 以降の意味型は `TypeId` / `MirValueType` / Semantic Type Spine が所有する。

残っていた判断は、GC aggregate の具体表現、defined type index の owner、semantic value と
storage の nullability、および enum family の識別方法である。

実装フェーズと一時的な移行制限は
[`docs/plans/wasm-gc-implementation.md`](../plans/wasm-gc-implementation.md)、詳細仕様は
[`RFC-007`](../rfcs/007-memory64-gc-layout-and-wasi-boundary.md) を正本とする。

## 決定

### 1. 採択する aggregate layout

以下は heap / composite type の定義であり、use-site の nullability を含まない。

| 言語型 | compiler-private Wasm GC 表現 |
|--------|--------------------------------|
| `String` | packed UTF-8 byte array。semantic value は non-null かつ言語上 immutable |
| `Vec<T>` | `{ buffer: ref Array<StorageT>, len: i32 }`。buffer は non-null |
| user struct | instantiated `TypeId` ごとの exact typed GC struct |
| enum | immutable tag を持つ base と final variant subtype |
| `Option` / `Result` | user enum と同じ規則 |

`Vec<T>` の capacity は `array.len(buffer)` から得る。独立した `cap` field は持たない。
`None` を null で表さず、payload-free の non-null variant object とする。

enum payload は immutable な exact storage type とする。ref を linear-memory address に変換せず、
scalar を一律 `anyref` box に格納しない。

### 2. semantic value と storage lowering を分離する

`lower_value_type` は function signature、引数、戻り値、semantic local などの値型を下げる。
`lower_storage_type` は aggregate field、Vec backing element、default 初期化が必要な storage など、
利用文脈を明示的に受け取って storage type を下げる。

`T` が non-null ref の場合、`Vec<T>` の backing element は同じ heap type の nullable ref とする。

- `[0, len)` の要素は必ず semantic `T` を満たす。
- `[len, capacity)` は未使用の null slot でよい。
- `get` は bounds check 後に `ref.as_non_null` で semantic `T` を復元する。
- `push` / `set` は non-null `T` だけを受け入れる。

これにより、default 値を持たない non-null ref element に `array.new_default` を適用しない。
enum payload や user struct field のように明示値で構築できる storage は exact type を維持する。

### 3. `TypeSectionPlan` を defined type index の唯一の owner にする

function type、struct、array、enum family、recursive group を含む全 defined type index は、
module-wide の `TypeSectionPlan` が一度だけ割り当てる。

`GcLayoutTable` は aggregate layout の materialized view であり、独自に type index を割り当てない。
type section、function signature、local、constructor、field access、canonical ABI adapter は、
同じ `TypeSectionPlan` と `GcLayoutTable` の結果を参照する。

名前 prefix、discovery order、文字列化した signature、`gc_type_base + offset`、body stack scan から
type index を復元してはならない。移行中の fallback は観測可能にし、production path から削除する。

### 4. aggregate member を `GcLayoutKey` で識別する

layout entry の key は次とする。

```text
GcLayoutKey {
    type_id: TypeId,
    member: GcMemberKey,
}
```

`GcMemberKey` は少なくとも aggregate 本体、`EnumBase`、`EnumVariant(VariantId)` を区別する。
variant 名や発見順を identity に使わない。同じ key の再登録は既存 entry を返し、同じ key に
異なる layout を要求した場合は internal compiler error とする。

### 5. heap layout と nullability を分離する

heap type identity は `GcLayoutKey` が、use-site nullability は `MirValueType` と lowering context が
所有する。例えば String heap type は byte array であり、通常の String value は `(ref $String)`、
scratch など必要な場所だけ `(ref null $String)` とする。

non-null から同じ heap type の nullable への widening は cast を発行しない。nullable から
non-null への narrowing は `ref.as_non_null` または明示的な型検査を必要とする。

### 6. source nominal identity を Wasm type index に委ねない

source-level identity は `TypeId` と Typed MIR verifier が所有する。Wasm の defined type index が
異なることだけを、source nominal identity の実行時保証として扱わない。

`ref.cast` / `br_on_cast` は、scrutinee の enum `TypeId` が静的に既知で、tag を確認した後の
同一 enum family 内 narrowing に限定する。`Any` や trait object からの一般 nominal downcast には
使用しない。一般 downcast が必要になった場合は brand / descriptor を別 ADR で決定する。

### 7. recursive group を決定的に並べる

`TypeSectionPlan` は defined type の依存グラフから strongly connected component（SCC）を作る。

1. 依存先 SCC を先に出力する。
2. 各 SCC を一つの recursive group にする。
3. explicit supertype を subtype より前に置く。
4. 残りは canonical `TypeId` key と `GcMemberKey` で安定順序化する。

### 8. host/component 境界

Memory64 の内部アドレスと component canonical memory の pointer width は別契約とする。
幅変換は型付き canonical ABI adapter が所有し、通常の MIR call site に無検査 truncate を置かない。

canonical memory を guest Memory64 と共有する範囲や adapter-owned memory32 buffer の詳細は
RFC-007 / issue #714 で決める。これらの実装完了は本 ADR の採択条件ではない。

## 帰結

- raw GC layout と binary shape に互換性保証は付かない。stable WIT / Canonical ABI は維持する。
- `wasm32` は linear representation を維持するが、`TypeId`、Semantic Type Spine、verifier を共有する。
- `lower_value_type` と `lower_storage_type` の責務を混ぜない。
- 一つの module 内で新旧 layout owner を function 単位に混在させない。
- invalid `TypeId`、missing layout、型競合を `i32` や open ref type へ fallback しない。
- 現行実装との差は current-state、plan、issue に記録し、ADR の決定を実装済みと扱わない。

## 却下した代替案

### call site ごとの `i32.wrap_i64`

範囲外アドレスを黙って truncate し、WASI P2 の interface / resource semantics を pseudo core
import に固定するため却下する。

### `GcLayoutTable` に独立した type index allocator を持たせる

`TypeSectionPlan` と owner が重複し、function type と aggregate type の index 空間が再び分裂するため
却下する。

### `(TypeId, layout kind)` だけで enum family を識別する

同じ enum `TypeId` の複数 variant を区別できないため却下する。`VariantId` を含む
`GcMemberKey` を使う。

### `Vec<T>` に `cap` field を持たせる

`array.len(buffer)` と `cap` の同期 invariant が増えるだけなので却下する。

### Vec backing element を常に semantic value type と同じにする

non-null ref に default 値がなく、未使用 capacity slot を初期化できないため却下する。

### Wasm type index を source nominal identity とする

Wasm の recursive type equivalence は type index 空間から独立しているため却下する。

### enum payload の linear-memory 併用を恒久化する

payload の ref / scalar 判定を call site で再推論することになり、ADR-002 / ADR-040 と衝突するため
却下する。

## 再検討条件

- Wasm GC の型同値性または default initialization 規則が変わる。
- `Any` / trait object の一般 nominal downcast を言語機能として導入する。
- stable raw Wasm GC ABI を公開する。
- `Vec<T>` の別表現が、型安全性を維持したまま実測で明確な利益を示す。

## 関連

- [ADR-002](ADR-002-memory-model.md) — GC 意味論
- [ADR-006](ADR-006-abi-policy.md) — compiler-private layout と stable WIT / Canonical ABI
- [ADR-008](ADR-008-component-wrapping.md) — in-tree component generation
- [ADR-013](ADR-013-primary-target.md) — primary target
- [ADR-040](ADR-040-typed-mir-signature-registry.md) — Semantic Type Spine
- [RFC-007](../rfcs/007-memory64-gc-layout-and-wasi-boundary.md) — 詳細設計
- [Wasm GC 実装計画](../plans/wasm-gc-implementation.md)
- [WebAssembly: instruction validation](https://webassembly.github.io/spec/core/valid/instructions.html)
- [WebAssembly: type syntax](https://webassembly.github.io/spec/core/syntax/types.html)
- [WebAssembly: validation conventions](https://webassembly.github.io/spec/core/valid/conventions.html)
- [WebAssembly: type validation](https://webassembly.github.io/spec/core/valid/types.html)
