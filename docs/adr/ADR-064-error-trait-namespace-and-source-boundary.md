# ADR-064: `Error` trait の名前空間と静的 source 境界

ステータス: **ACCEPTED** — stable な既存 `Error` enum を維持しつつ、静的 dispatch 用の `Error` trait を導入する

決定日: 2026-09-19

関連: [ADR-014](ADR-014-stability-labels.md)、[ADR-036](ADR-036-trait-stdlib-redesign.md)、[ADR-062](ADR-062-language-v2-trait-core-coherence.md)、[ADR-063](ADR-063-language-v2-conversion-error-boundary.md)、[#694](../../issues/open/694-error-trait-unified-ecosystem.md)

## 文脈

`std::core::error` には、stdlib の既存 API と fixture が利用する `Error` enum と
`error_message` があり、manifest でも stable として公開されている。
一方、#694 は `Display` を supertrait とする `Error` trait を必要とする。
同じモジュールで enum と trait に同じ名前を割り当てると、ADR-014 の告知付き移行を
経ずに stable API を壊すことになる。

さらに、現行の言語 v2 は静的 trait dispatch を採択しており、`dyn Trait` の object ABI
はまだ存在しない。
Rust の `source() -> Option<dyn Error>` をそのまま導入すると、未採択の object 表現を
stdlib の意味論へ持ち込む。

## 決定

### 1. trait の名前空間

- 既存の `std::core::error::Error` enum と `error_message` は現行 stable surface として
  維持する。
- 新しい trait は `std::error::Error` として公開する。trait 自体の安定性は、typed
  source object ABI が未確定であるため `provisional` とする。
- `std::core::error::Error` を `AppError` へ改名する案は、ADR-014 の deprecation と
  migration release を別途完了するまで採用しない。

### 2. supertrait と dispatch

- trait 宣言は `trait Error: Display { ... }` のように一つ以上の supertrait を持てる。
- `impl Error for T` はすべての supertrait の impl を要求する。
- supertrait のメソッドは静的 dispatch の bound lookup から利用できる。
- blanket impl と `dyn Error` の object ABI はこの決定に含めない。

### 3. `source` の返り値

- 現行の静的 ABI では `Error::source(self: Error) -> Option<String>` を正規形とする。
  `Some` は原因の人間向け表示、`None` は根本原因を表す。
- `Option<dyn Error>` や typed source chain は、object ABI と所有権境界を別 ADR で採択する
  まで導入しない。
- `Display` はユーザー向け表示、`Error::source` は原因表示を担当し、内部 compiler
  sentinel や raw error code を source payload として公開しない。

### 4. target parity

trait の合法性、supertrait obligation、`?` が使う `From` conversion は target 非依存とする。
`wasm32-gc` と `wasm32` linear-memory fallback は同じ source/typechecker 結果を共有し、
`Option<String>` の所有権と値意味論も同一とする。

## 帰結

- `String` は parse error の共通表示型として `Error` を実装できる。
- `std::io::IoError` などの concrete error は `Display` と `Error` を明示的に実装する。
- 既存の `std::core::error::Error` enum を利用するコードは移行なしで動作し続ける。
- typed source chain が必要になったときは、stable enum の移行期間と object ABI を先に決める
  必要がある。

## 却下した代替案

### `std::core::error::Error` を直ちに trait へ置換する

stable enum、既存 fixture、`error_message` の利用を同時に破壊するため、ADR-014 に反する。

### trait 名を `StdError` にする

既存の Rust-compatible な API 期待と #694 の契約から不必要に離れ、将来の移行 alias を増やす。

### `Option<dyn Error>` を先に導入する

ADR-062 の静的 dispatch 境界と未採択の object ABI を曖昧にするため採用しない。

## 再検討条件

- `dyn Trait` の object ABI と root/lifetime 規則を採択したとき。
- ADR-014 に従う `std::core::error::Error` enum の deprecation release と移行ガイドが完了したとき。
- typed source chain が `wasm32-gc` と linear-memory fallback で同一意味論を証明できるとき。
