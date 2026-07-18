# Wasm GC 実装計画

ステータス: 実装計画（決定記録ではない）
関連 ADR: ADR-035（ACCEPTED） / ADR-040（ACCEPTED）
詳細設計: [RFC-007](../rfcs/007-memory64-gc-layout-and-wasi-boundary.md)

---

## ゴール

`wasm32-gc` ターゲットで既存 fixture suite が全通過し、生成物を
`wasm-tools validate --features gc` が受理すること。

現行実装には GC 命令、部分的な GC String / Vec / struct、Typed MIR spine が既にある。
以下はそれらを作り直す計画ではなく、ADR-035 の ownership と型安全性へ移行する順序である。

## 正規の実装順序

```text
Phase 1: TypeSectionPlan を全 defined type index の owner にする
  ↓
Phase 2: value/storage lowering を分離し、Vec backing を移行する
  ↓
Phase 3: enum / Option / Result family を VariantId ベースへ移行する
  ↓
Phase 4: WASI P2 canonical boundary adapter を整備する
  ↓
Phase 5: Typed MIR verifier を hard gate 化し、fallback を削除する
```

Typed MIR / layout verifier の observation は Phase 1 から全フェーズで有効にする。
ただし missing layout を hard error にするのは、production fallback を除去する Phase 5 とする。

## 前提として維持する既存基盤

- `MirValueType`、`TypeId`、`SignatureRegistry`、`MonoInstanceTable` を semantic spine とする。
- GC opcode / writer、struct / array 命令、target dispatch を再利用する。
- String heap type は packed UTF-8 byte array とする。
- `wasm32` は同じ言語意味論の linear lowering を維持する。
- GC layout を stable ABI にせず、公開相互運用は WIT / Canonical ABI を維持する。
- emitter 内で `TypeTable` に新しい `TypeId` を intern しない。

## Phase 1: TypeSectionPlan owner

function type、struct、array、enum family、recursive group を含む全 defined type index を、
module-wide の `TypeSectionPlan` が一度だけ割り当てる。

### 実装内容

- `TypeTable`、`SignatureRegistry`、instantiated `TypeId`、aggregate definition から依存グラフを作る。
- SCC ごとに recursive group を作り、依存先 SCC を先に出力する。
- explicit supertype を subtype より先にする。
- 残りを canonical `TypeId` key と `GcMemberKey` で安定順序化する。
- `GcLayoutTable` は `TypeSectionPlan` が割り当てた aggregate index の materialized view にする。
- `GcLayoutKey { type_id, member }` を導入し、aggregate 本体、enum base、variant を区別する。
- signature、local、constructor、field access が plan 由来の同じ entry を参照できるようにする。
- 現行経路と plan の expected / actual type を observation verifier へ記録する。

### このフェーズでは残すもの

- 名前 / fixed offset fallback は観測付きで残す。
- enum payload の linear representation は変更しない。
- WASI import pointer width は変更しない。

### 完了条件

- 同じ module input から defined type index と recursive group が決定的に再現される。
- 同じ `GcLayoutKey` への同一 layout 再登録は同じ entry を返す。
- 同じ key への異なる layout 要求は diagnostic context を持つ internal compiler error になる。
- type identity / nullability lane の対象 fixture が regression なく改善する。

## Phase 2: value/storage lowering と Vec

semantic value の型と、default initializationを必要とする storage の型を分離する。

### 実装内容

- `lower_value_type` を function signature、call、return、semantic local の入口にする。
- context-aware な `lower_storage_type` を aggregate field、Vec backing、scratch local の入口にする。
- context は bool ではなく、利用目的を表す分類値にする。
- String の heap type と use-site nullability を分離する。
- `Vec<T>` を `{ buffer: ref Array<StorageT>, len: i32 }` に統一する。
- capacity は `array.len(buffer)` から取得し、独立した `cap` field を追加しない。
- `T` が non-null ref のとき、`StorageT` は同じ heap type の nullable ref にする。
- `Vec::get` は bounds check 後に `ref.as_non_null` で semantic `T` を復元する。
- `push` / `set` は semantic non-null `T` だけを受け入れる。

### 完了条件

- `array.new_default` を default 値のない non-null ref element typeへ発行しない。
- `[0, len)` と `[len, capacity)` の invariant を verifier observation で区別できる。
- signature、semantic local、Vec backing が同じ lowering context を誤用していない。
- String / Vec / HashMap lane の対象 fixture が regression なく改善する。

## Phase 3: enum family

enum / `Option` / `Result` を、base と final variant subtype からなる一つの familyへ移行する。

### 実装内容

- `GcMemberKey::EnumBase` と `GcMemberKey::EnumVariant(VariantId)` を plan に登録する。
- base の immutable discriminant field と variant の immutable exact payload field を定義する。
- payload-free variant も non-null object として生成する。`None = null` は採用しない。
- constructor、call、return、match bind が同じ variant layout を参照する。
- match は tag 確認後だけ、静的に既知の enum family 内で `ref.cast` / `br_on_cast` を行う。
- ref payload を linear address に変換せず、scalarを一律 `anyref` box に格納しない。

### 完了条件

- enum payload lane の対象 fixture で `i64 payload ↔ GC ref` の混同がなくなる。
- variant identity が名前や discovery order に依存しない。
- user enum、`Option`、`Result` が同じ layout 規則を使う。
- 一般 nominal downcast に Wasm type index を流用していない。

## Phase 4: WASI P2 canonical boundary

GC layout laneとは分離し、ADR-035 が固定した「幅変換の owner」を component adapter に実装する。

### 実装内容

- `HostIntrinsicSpec` に Ark-side signature と canonical boundary signature を持たせる。
- pointer width を target 名ではなく、adapter が使用する canonical memory の index type から決める。
- Memory64 から memory32 への narrowing は adapter 内で range check または adapter-owned buffer copy を行う。
- pseudo core import を issue #714 の component-correct interface / resource adapterへ移行する。
- 通常の MIR call site には無条件 `i32.wrap_i64` を追加しない。

### 完了条件

- `host_module_contract` が core Wasm と component の両方で validate される。
- pointer narrowing が canonical adapter 外に存在しない。
- `python3 scripts/manager.py verify component-interop` が通る。

## Phase 5: verifier hard gate と cleanup

Phase 1–4 の observation 結果を使い、invalid Wasm を生成する前に型不一致を止める。

### 実装内容

- local assignment、call signature、field access、nullability、lowering context を検証する。
- lowering recipe の input / output arity と stack effect を検証する。
- expected / actual に function、instruction、`TypeId`、repr、nullability、storage context を含める。
- missing `TypeId` / layout、layout conflict、unchecked pointer narrowing を hard error にする。
- 名前 prefix、`gc_type_base + offset`、string signature canonicalization、stack scan fallback を削除する。
- invalid type の `i32` / open ref fallback を production path から削除する。

### 完了条件

- fallback observation count が全対象 fixture でゼロになる。
- `hash_trait` のような stack underflow を Wasm validation 前に検出する。
- T3 Wasm validation gate と `verify quick` が通る。
- targeted fixture 通過後に `selfhost fixpoint` で stage-2 / stage-3 の一致を確認する。

## PR 境界と依存関係

- Phase 1–3 は同じ type section / emitter ownerを変更するため直列化する。
- Phase 4 は Phase 1 の typed signature契約後なら別レーンで進められるが、正規順序では Phase 3 後に統合する。
- Phase 5 の observation 実装は先行できるが、hard gate と fallback削除は Phase 1–4 完了後に行う。
- 一つの PR で複数フェーズの完了を主張しない。各フェーズの完了条件を個別に記録する。
- compiler Wasm の再構築は編集をまとめて一回行い、その成果物で対象 fixture をまとめて検証する。

## 検証コマンド

```bash
python3 scripts/manager.py fmt --check
python3 scripts/manager.py selfhost build-compiler
python3 scripts/check/check-t3-wasm-validate.py
python3 scripts/manager.py verify component-interop
python3 scripts/manager.py verify quick
python3 scripts/manager.py selfhost fixpoint
```

`build-compiler` は emitter の編集をまとめた後に一回だけ実行する。`selfhost fixpoint` は targeted
fixture が通った後の ADR-029 gate として使い、日常の rebuild に使わない。

## リスクと未決事項

1. recursive group の binary encoding 実装は既存 type signature canonicalization と広く競合する。
2. `wasm32` / `wasm32-gc` は lowering を分けるが、言語意味論を分岐させてはならない。
3. canonical memory を guest Memory64 と共有する範囲は RFC-007 / issue #714 に残る。
4. non-null function local の最適化は型安全な baseline の後に行う。
5. Wasm GC performance は fixture parity 達成後に benchmark gate で評価する。

## スコープ外

- stable raw Wasm GC ABI
- `Any` / trait object の一般 nominal downcast
- Post-MVP GC features、`Weak<T>`、finalizer
- WASI P3 async-first
- native backend ABI

## 関連

- [ADR-035: Wasm GC 内部レイアウト方針](../adr/ADR-035-wasm-gc-implementation.md)
- [ADR-040: Semantic Type Spine](../adr/ADR-040-typed-mir-signature-registry.md)
- [RFC-007: Memory64 GC レイアウトと WASI P2 境界](../rfcs/007-memory64-gc-layout-and-wasi-boundary.md)
- [残存 validation failure 調査](../research/memory64-validate-fail-10.md)
- [Issue #808](../../issues/open/808-t3-wasm-validation-failures.md)
- [Issue #714](../../issues/open/714-wasi-p2-emitter-native-component-output.md)
