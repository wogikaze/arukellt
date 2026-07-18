# RFC-007: Memory64 GC レイアウトと WASI P2 境界

ステータス: DRAFT

関連 ADR: [ADR-035](../adr/ADR-035-wasm-gc-implementation.md)（PROPOSED） /
[ADR-040](../adr/ADR-040-typed-mir-signature-registry.md)（ACCEPTED）

関連 issue: [#808](../../issues/open/808-t3-wasm-validation-failures.md) /
[#714](../../issues/open/714-wasi-p2-emitter-native-component-output.md)

提案日: 2026-07-18

本 RFC は `wasm32-gc` + Memory64 で残る validation failure に対し、GC type identity、
enum / `Option` / `Result`、nullability、WASI P2 import ABI の境界を一つの設計へ整理する。
現行挙動の正本ではなく、ADR-035 が採択されるまで layout 部分は設計提案である。

---

## 要約

1. `TypeId` から module-wide の `GcTypePlan` を一度だけ構築し、type section、function
   signature、local、constructor、field access が同じ type index を参照する。
2. enum / `Option` / `Result` は共通 base と variant subtype からなる GC struct とし、
   payload は具象 `MirValueType` から得た exact Wasm 型で保持する。
3. nullability は GC type identity ではなく use-site の値型属性とする。nullable widening に
   `ref.cast` を使わず、narrowing と variant downcast だけを明示する。
4. Memory64 の内部アドレスと component canonical memory の pointer width は別契約とする。
   変換は `HostIntrinsicSpec` / canonical ABI adapter が所有し、通常 call site では行わない。
5. Wasm emit 前に Typed MIR verifier を hard gate とし、型不一致、未定義 stack effect、
   無検査 pointer narrowing を invalid Wasm ではなく compiler error として検出する。

## 問題

[`memory64-validate-fail-10.md`](../research/memory64-validate-fail-10.md) の 10 件は、
表面上は四種類の validation error に見える。

| 症状 | 代表 fixture | 設計上の原因 |
|------|--------------|--------------|
| `i64` pointer を `i32` import へ渡す | `host_module_contract` | guest memory と host/component ABI の境界 owner がない |
| `i64` payload と GC ref の混同 | `json_perf_decode`, `buf_read`, `ord_sort_by` | enum container と payload の表現が Typed MIR で確定していない |
| 同じ見かけの ref type が不一致 | `hashmap_generic_demo`, `io_copy`, TOML 2 件 | type section と body が別経路で type index を復元している |
| stack underflow | `hash_trait` | 命令の入力型・stack effect が emit 前に検証されていない |

`wit_type_basic` と TOML 2 件は error text に nullability 差が現れるが、単に
`ref null` cast を追加する問題ではない。constructor の heap type と local の heap type が
一致していることを先に保証し、その上で nullability の widening / narrowing を扱う必要がある。

## 設計目標

- Typed MIR 以降で名前、mangled suffix、直前の stack、固定 type offset から型を推測しない。
- 一つの具象 Ark 型が一つの core module 内で一意な Wasm GC type family を持つ。
- GC aggregate の field、call、local assignment を Wasm validation と同じ規則で emit 前に検査する。
- `wasm32` の linear lowering と `wasm32-gc` の GC loweringで言語意味論を分岐させない。
- stable interop surface は WIT / Canonical ABI のままにし、GC layout を公開 ABI にしない。
- WASI P2 の interface / resource 呼び出しを pseudo core import の signature に固定しない。

## 非目標

- raw Wasm GC ABI の安定化
- `wasm32` linear-memory backend の削除
- WASI P3 async、Weak reference、finalizer、post-MVP GC 機能
- #714 が対象とする全 WASI capability の実装
- validation 件数を ADR / RFC 内の固定値として管理すること

## 詳細設計

### D1: `GcTypePlan` を type identity の唯一の owner にする

現行 `GcLayoutTable` は type section の生成後に `gc_type_base + offset`、名前 prefix、
struct / variant map を組み合わせて index を復元している。この順序を反転する。

```text
TypeTable + instantiated TypeId + aggregate definitions
    -> GcTypePlan（決定的順序で layout と type index を割当）
        -> type section
        -> function signatures
        -> locals
        -> instruction emission
        -> canonical ABI adapters
```

`GcTypePlan` は既存 `GcLayoutTable` を置換する別 registry ではない。移行後の
`GcLayoutTable` が plan の materialized view となる。owner を二つ作らない。

各 entry は最低限次を保持する。

- instantiated `TypeId`
- layout kind: string array / vec struct / user struct / enum base / enum variant
- canonical heap type index
- supertype index（variant のみ）
- field ごとの `MirValueType` と `WasmValueType`
- constructor result の nullability

登録 key は `(TypeId, layout kind)` とする。同じ `TypeId` に同じ kind を二度登録した場合は
既存 entry を返す。異なる layout kind を要求した場合は internal compiler error とする。
type index の順序は TypeTable / definition order と型引数の canonical `TypeId` 列から決め、
名前 prefix や discovery order に依存させない。

次を production fallback として残さない。

- `gc_type_base + 14` のような type family offset
- `Option_` / `result:` / `vec:` prefix からの index 復元
- stack scan による ref local type 推論
- emit 中の `TypeTable_intern`

移行中は fallback の利用箇所を計測できるようにし、hard gate 化する前にゼロにする。

### D2: enum / `Option` / `Result` は base + variant subtype

具象 enum `E` ごとに base type と variant type を同じ plan entry family として作る。

```wat
(type $E (sub (struct
    (field i32))))
(type $E.A (sub final $E (struct
    (field i32))))
(type $E.B (sub final $E (struct
    (field i32)
    (field <payload-0>)
    (field <payload-1>))))
```

実際の field mutability と type index は compiler-private である。ただし次の不変条件を持つ。

- base の先頭 field は discriminant で、全 variant が同じ型・位置で継承する。
- payload field は instantiated variant definition の `MirValueType` から lowering する。
- ref payload を `i32` / `i64` address として格納しない。
- scalar payload を `anyref` box に一律変換しない。
- payload-free variant も enum value として有効な non-null object を作る。
- `Option<T>` と `Result<T, E>` は builtin 特例の別表現を持たず、同じ規則を使う。

match は base の tag を読み、payload arm に入った後だけ該当 variant へ downcast する。
constructor、function return、match bind は同じ variant entry の field layout を参照する。
`None = null` の最適化は nested `Option` と user enum の規則を分岐させるため、本 RFC では採らない。

### D3: nullability は use-site 属性

heap type identity と nullability を分ける ADR-040 の `MirValueType` をそのまま使う。

- constructor result: non-null
- enum / struct の semantic value: non-null
- default initialization が必要な scratch / migration local: nullable を許可
- nullable から non-null への narrowing: `ref.as_non_null` または型付き check
- enum base から variant への narrowing: `ref.cast` / `br_on_cast`
- non-null から同一 heap type の nullable への widening: cast を発行しない

Wasm local を nullable にする都合を `TypeId` へ混ぜない。全 ref を nullable に固定するのも、
constructor ごとに別の non-null heap type を作るのも禁止する。Typed MIR verifier は
heap type identity と nullability を別々に比較する。

### D4: function signature、local、field は同じ lowering 関数を使う

`lower_mir_value_type(plan, mvt)` を次の全箇所の唯一の変換入口にする。

- direct / indirect call signature
- function param / result
- local declaration
- struct / enum field
- `local.set` / `local.tee`
- call argument / return assignment
- canonical ABI adapter の Ark-side signature

legacy `val_type + type_name` は migration input としてのみ許し、body emitter がそれを
再解釈しない。`TypeId` が invalid、layout entry がない、field index が範囲外の場合は、
`i32` や enum-open type に fallback せず emit 前に internal compiler error とする。

### D5: host/component 境界は二つの ABI を明示する

host call には少なくとも次の二つの signature がある。

1. Ark-side signature: GC ref、enum、scalar からなる `SignatureEntry`
2. boundary signature: WIT を flatten した canonical ABI の scalar / pointer / handle

`HostIntrinsicSpec` は名前と return の概略分類だけでなく、両 signature、canonical
memory、pointer width、adapter `FunctionId` を保持する。

```text
Ark code
  -> Ark-side typed call
  -> generated canonical adapter
       GC String/Vec/Result <-> canonical memory/list/result/resource
       checked address-width conversion（必要な場合だけ）
  -> component import
```

pointer type は target 名から推測せず、adapter が参照する canonical memory の index type
から決める。同じ Memory64 memory を canonical memory とする場合は `i64`、memory32 adapter
を使う場合は `i32` である。後者で Memory64 address を渡す必要があるなら、adapter 内で
上限を検査するか adapter-owned buffer へ copy する。通常 call site の無条件
`i32.wrap_i64` は認めない。

`wasi:cli/stdout@0.2.0::write` や `wasi:cli/environment@0.2.0::args-sizes` のような
pseudo core import を最終 ABI としない。#714 に従い、WASI interface function、resource、
canonical lift/lower を component emitter が表現する。validation failure を止めるためだけの
pseudo import widening / narrowing は production architecture にしない。

### D6: Typed MIR verifier を Wasm emit 前の hard gate にする

verifier は CFG ごとに少なくとも次を検査する。

- 各 instruction operand と result に valid `MirValueType` がある。
- `local.set` / `local.tee` の source と destination が代入可能である。
- call args / results が `SignatureRegistry` / `MonoInstanceTable` と一致する。
- struct / enum field access が `GcTypePlan` の owner と field type に一致する。
- ref narrowing は明示命令を伴い、nullable widening は identity を変えない。
- stack effect を持つ Wasm lowering recipe の input / output arity が宣言と一致する。
- pointer narrowing は canonical adapter 内の checked operation に限る。

失敗時は invalid Wasm を出力せず、function identity、MIR instruction、expected / actual
`TypeId`、repr、nullability を含む internal compiler error にする。ユーザー入力から到達する
場合の CLI/LSP 処理は ADR-015 に従い panic しない。

## 10 fixture の切り分け

| レーン | fixture | 主な修正 owner | 独立性 |
|--------|---------|----------------|--------|
| Type identity | `hashmap_generic_demo`, `io_copy`, `wit_type_basic`, TOML 2 件 | D1–D4 | enum payload 移行前に修正可能 |
| Enum payload | `json_perf_decode`, `buf_read`, `ord_sort_by` | D2–D4 | GcTypePlan 後に実施 |
| Body stack | `hash_trait` | D6 + 該当 lowering recipe | verifier 導入後に独立修正可能 |
| WASI boundary | `host_module_contract` | D5 / #714 | GC layout と別レーン。ただし typed signature 基盤を共有 |

10 件を一括の emitter patch にしない。D1/D4/D6 が共通の先行基盤で、enum と WASI は
その後に別々に進められる。

## 代替案

### A. validation offset ごとの cast / wrap 追加

却下。症状を別の fixture へ移し、型 identity と pointer ownership を確立しない。

### B. 既存の固定 GC type family offset を一箇所へ集約

却下。重複コードは減るが、type section と semantic type の owner が分かれたままである。

### C. enum を linear-memory tagged union のまま残す

却下。GC target 内で payload の ref / scalar 判定を失い、ADR-002 / ADR-040 と衝突する。
`wasm32` backend の linear loweringとしては維持する。

### D. host import signature を Memory64 に合わせてすべて `i64` にする

却下。component 側の canonical memory や resource ABI を無視し、consumer が期待する
core signature と一致する保証がない。境界 signature は WIT と canonical options から導出する。

## 移行と互換性

- GC layout は compiler-private なので、raw GC type index の互換性 migration note は不要。
- stable WIT / Canonical ABI の挙動は維持する。adapter の binary shape は非公開実装詳細である。
- `wasm32` は既存 linear layout を維持するが、TypeId / SignatureRegistry / verifier は共有する。
- migration 中に新旧 GC layout を function 単位で混在させない。一つの module は一つの
  `GcTypePlan` を使う。
- current-state の partial 表記は、target gate が実測で通るまで変更しない。

## 未決事項

- Wasm GC recursive type group が必要な相互再帰型の deterministic ordering
- canonical memory を guest Memory64 と共有する範囲と、memory32 adapter-owned buffer の範囲
- non-null function local を直接使える場合の最適化時期
- payload field mutability と variance の最終 binary encoding

これらは実装着手前に probe fixture で確定する。未決事項を理由に、TypeId owner、
無検査 truncate 禁止、enum payload exact typing を弱めない。

## 関連

- [調査: 10 remaining Memory64 failures](../research/memory64-validate-fail-10.md)
- [ADR-006: 公開 ABI 境界](../adr/ADR-006-abi-policy.md)
- [ADR-008: Component Model wrapping](../adr/ADR-008-component-wrapping.md)
- [ADR-035: Wasm GC layout](../adr/ADR-035-wasm-gc-implementation.md)
- [ADR-040 / RFC-002: Semantic Type Spine](002-semantic-type-spine.md)
- [Wasm GC 実装計画](../plans/wasm-gc-implementation.md)
