# ADR-060: 言語 v2 の owned ADT・Box・Slice

ステータス: **ACCEPTED** — `Option` / `Result` / enum、`Box<T>`、`Slice<T>` の値意味論と backend 境界を採択する

決定日: 2026-09-19

関連: [ADR-002](ADR-002-memory-model.md)、[ADR-013](ADR-013-primary-target.md)、[ADR-035](ADR-035-wasm-gc-implementation.md)、[ADR-058](ADR-058-language-v2-semantics-redesign.md)、[ADR-059](ADR-059-language-v2-ownership-copy-mutability.md)、[RFC-011](../rfcs/011-language-v2-semantics-redesign.md)、[#854](../../issues/open/854-adt-box-slice-value-semantics.md)

## 文脈

ADR-059 は binding の move、明示的な `Copy`、`let mut` を採択した。
しかし、`Option` / `Result` / enum の payload、heap 上の値を表す `Box<T>`、
`Vec` の一部を読む view である slice について、所有権の境界と backend の表現がまだ同じ契約に接続されていなかった。

このままでは、Wasm GC の tagged object と `wasm32` の linear-memory 表現が、型検査や暗黙コピーの規則を別々に持つ余地が残る。
また、`Box` の unwrap や slice の作成を単なる backend intrinsic として扱うと、移行中の stdlib が値の所有者を失う。

## 決定

### 1. `Option` / `Result` / enum は owned tagged ADT とする

- `Option<T>` は `None` または `Some(T)`、`Result<T, E>` は `Ok(T)` または `Err(E)` を持つ owned ADT である。
- ユーザー定義 enum も variant tag と payload を持つ owned ADT である。
- ADT の variant を構築・分解するときは、payload の通常の move / `Copy` 規則を適用する。
- `Option<T>` と `Result<T, E>` は、すべての payload が `Copy` の場合に限り構造的に `Copy` である。
- ユーザー定義 enum は、全 payload が `Copy` であっても `impl Copy for T {}` がなければ `Copy` ではない。
- tag、payload の unboxed 化、GC object の種類、linear-memory の配置は backend detail であり、source-level の所有権や `Copy` 判定を変更してはならない。

### 2. `Box<T>` は一意所有の owned value とする

- `Box<T>` は heap 上の `T` を一意に所有する非 `Copy` の値である。
- `Box_new(value)` は、`value` が非 `Copy` ならその binding を消費して `Box<T>` を作る。
  `Copy` 値については ADR-059 の通常の copy 規則を適用する。
- `unbox(boxed)` は `Box<T>` を消費し、所有されていた `T` を返す。
- GC の object reference は collector への到達性を表すだけであり、`Box<T>` の暗黙共有や暗黙複製を意味しない。

### 3. `Slice<T>` は非 `Copy` の読み取り view とする

- `Slice<T>` は source collection と境界付きの range を保持する非 `Copy` の view である。
- `as_slice(source, start, end)` は source の現在の owner binding を消費せず、source の backend object と range を参照する view を作る。
  範囲外の start/end は既存の collection access と同じ bounds-check 契約に従う。
- `slice_len(view)` は view の要素数を返し、`slice_get(view, index)` は view 内の要素を返す。
  これらの API は現在の stdlib の引数契約に従い、view の値を消費する。
- `Slice<T>` の view は読み取り API だけを公開する。source の mutation と view の同時利用を静的に排他する完全な lifetime / borrow checker はこの ADR の範囲外であり、後続の closure / borrow work で採択する。
- source owner が生きていること、view が範囲を越えて要素を読むこと、backend が異なることを理由に source-level の意味を変えてはならない。

### 4. GC と linear-memory は同じ契約を lower する

- `wasm32-gc` は GC `struct` / `array` / `ref` と tagged/unboxed representation を優先して使用できる。
- `wasm32` は linear-memory representation と既存 runtime を使用できる。
- 両 target は、owned ADT の move / `Copy`、`Box` の一意所有、`Slice` の source 非消費と range 読み取りについて同じ typecheck result、diagnostic、observable output を提供する。
- collector、header、tag layout、reference width、name mangling、ABI は compiler-private であり、この ADR はそれらの bytes を固定しない。
- `wasm32` に tracing GC fallback を段階導入する作業は、#854 の semantic contract と独立した後続 issue (#873/#874) で検証する。

## 帰結

- ADT の payload を含む move / `Copy` の判定は、GC object か linear value かではなく source type と `impl Copy` から決まる。
- `Box_new` と `unbox` は compiler intrinsic の別意味論ではなく、stdlib の owned ADT API として typechecker / lowerer から同じ境界で扱われる。
- slice の source binding を保持したまま view を作れるため、既存の collection migration は source を明示的に clone するだけでは済まない。
  mutation と escape の静的規則は、後続の借用設計でこの契約を拡張する。
- #854 の positive / negative fixture は `wasm32-gc` と `wasm32` の両方で compile、validate、runtime parity を確認する。

## 却下した代替案

- **GC reference をすべて `Copy` とみなす。** `Box` と ADT の所有境界を失い、linear-memory と異なる意味論になるため採用しない。
- **slice 作成時に source を必ず move する。** 現在の view API が表す source 非消費の契約と移行中の stdlib を壊すため採用しない。
- **GC target と linear target で別の bounds / move 規則を持つ。** target-dependent な source semantics を作るため採用しない。
- **slice の lifetime checker をこの ADR で先取りする。** closure capture、mutation、escape の未決定事項を一つの ADT 判断へ混ぜるため採用しない。

## 再検討条件

次のいずれかが発生した場合に再検討する。

- 完全な borrow / lifetime 規則が `Slice<T>` の source 非消費契約と衝突する。
- Component または stable raw Wasm ABI が、tagged/unboxed layout を公開契約として要求する。
- GC と linear-memory の parity fixture が同じ source program の診断または observable output を維持できない。
