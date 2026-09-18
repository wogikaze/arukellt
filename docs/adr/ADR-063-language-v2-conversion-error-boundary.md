# ADR-063: Language v2 の変換 trait と error sentinel の境界

ステータス: **ACCEPTED** — `?`、変換 trait、compiler 内部 error sentinel の境界を固定する

決定日: 2026-09-19

関連: [ADR-039](ADR-039-question-mark-operator.md)、[ADR-062](ADR-062-language-v2-trait-core-coherence.md)、[ADR-014](ADR-014-stability-labels.md)、[RFC-011](../rfcs/011-language-v2-semantics-redesign.md)、[#857](../../issues/open/857-question-mark-conversion-error-sentinel.md)、[#694](../../issues/open/694-error-trait-unified-ecosystem.md)、[#708](../../issues/open/708-tryfrom-tryinto-traits.md)

## 文脈

Arukellt は `Result<T, E>` と `Option<T>`、`From<T>` / `Into<T>`、`?` の構文を持つ。
一方、fallible conversion の `TryFrom<T>` / `TryInto<T>` は別 issue で定義中であり、
compiler の型検査用 recovery sentinel と、ユーザーが宣言できる `Error` という nominal type の境界も
文書ごとに表現が揺れていた。

`?` がどの trait を暗黙に呼ぶかを曖昧にすると、失敗値の変換が import、宣言順、または backend 表現に
依存する。error sentinel をユーザー型として公開すると、型エラーを回復するための内部値が通常の
`Result` payload や trait obligation に混入する。

## 決定

### 1. Infallible と fallible conversion は別の契約である

- `From<T> for U` は `T` から `U` への infallible conversion を表す。
- `Into<U> for T` はその dual な公開呼び出し面を表す。`From<T> for U` が存在することだけを理由に、
  user-visible な `Into` impl を暗黙生成しない。必要な blanket rule は ADR-062 の compiler-owned rule として
  明示的に採択する。
- `TryFrom<T> for U` は `Result<U, E>` を返す fallible conversion、`TryInto<U> for T` はその dual な呼び出し面
  である。`TryFrom` / `TryInto` は `From` / `Into` の subtrait ではなく、片方の存在からもう片方や
  infallible conversion を暗黙に導出しない。
- `?` の異種 `Result` error conversion は canonical `From<E_source> for E_target` だけを使う。
  `TryFrom` / `TryInto` の探索や実行時 conversion へフォールバックしない。

この関係は、既存 stdlib の明示 impl と、後続の #708 による fallible conversion の追加を同じ coherence
規則へ接続しつつ、`?` の意味を変換の成功可能性から独立させる。

### 2. `?` の型検査境界

- `Result<T, E_source>?` は、囲む関数の `Result<_, E_target>` へ `T` を返し、`Err` だけを
  `From<E_source> for E_target` で変換する。同一型は identity とする。
- `Option<T>?` は、囲む関数の `Option<_>` へ `T` を返し、`None` を identity で伝播する。
- `Option` と `Result` の間を `?` で暗黙変換しない。
- operand、囲む関数の return type、または必要な `From` impl が不適合なら、lowering へ進めず
  typecheck diagnostic を出す。diagnostic の code は E0210 とし、宣言順を候補選択規則にしない。

### 3. Error sentinel は compiler 内部だけに置く

parser の `NK_ERROR`、型検査の unresolved/unknown state、CoreHIR/MIR の error-recovery sentinel は
compiler 内部の状態であり、source-level `TypeInfo`、trait impl、`Result` payload、公開 ABI の型として
解決してはならない。
ユーザーが宣言する `struct Error` / `enum Error` は通常の nominal type として解決し、内部 sentinel の
名前や tag と共有しない。
内部 sentinel が残ったまま code generation に到達した場合は、ユーザー型への暗黙変換ではなく compiler
diagnostic または phase failure とする。

### 4. Target parity

上記の conversion candidate、`?` の合法性、diagnostic、sentinel 境界は target 非依存である。
`wasm32-gc` と `wasm32` linear-memory fallback は同じ source/typechecker 結果を使い、From の Err rewrite
や Option/Result の representation の違いを合法性の違いにしない。

## 帰結

- `?` の型検査は MIR の callee lookup より前に必要な `From` obligation を確定する。
- `TryFrom` / `TryInto` の trait 定義、associated `Error`、標準 impl、fallible conversion の ABI は #708 の
  scope に残る。
- `std::core::error::Error` trait と library-wide error ecosystem は #694 の scope に残る。
- `Error` というユーザー型を禁止する必要はない。禁止するのは内部 recovery state の source-level 漏洩である。

## 却下した代替案

### `?` が `TryFrom` を優先して error を変換する

`?` の成功/失敗意味論が fallible conversion の実装へ依存し、失敗時の二重 `Result` と termination を
曖昧にするため採用しない。

### `From` の存在からすべての `Into` を暗黙生成する

coherence と候補数を registry の外へ隠し、既存の明示 impl と重複するため、明示的 compiler-owned rule
なしには採用しない。

### 内部 `Error` sentinel を組み込み source type として公開する

型エラー回復用の値がユーザーの型、trait、ABI に混入し、エラーを成功扱いにするため採用しない。

## 再検討条件

- #708 で fallible conversion の object/ABI と `TryFrom`→`TryInto` の compiler-owned blanket を採択する場合。
- #694 で `Error` trait と error source chaining の型表現が確定し、`?` の From obligation と統合する必要が生じた場合。
- error recovery sentinel を独立した typed compiler IR として表現でき、source type と機械的に分離できる場合。
