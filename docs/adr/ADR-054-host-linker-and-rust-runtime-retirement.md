# ADR-054: host-linker とリポジトリ内 Rust runtime の退役

ステータス: **ACCEPTED** — selfhost Wasm を Wasmtime と公式 WASI Component Model だけで実行する

決定日: 2026-09-12

## 文脈

selfhost compiler の実行には、Rust 製の `host-linker`、heap patcher、WASI P2 adapter、
および旧来の HTTP/TCP bridge が介在している。
これらは compiler が生成する Wasm の ABI と実行環境を別々に保ち、同じ入力に対して
実行経路を選ぶための状態と互換性コードを増やしている。
WASI Preview 2 は Component Model の公式 interface と canonical ABI を実行境界として
提供しているため、リポジトリ独自の host runtime を維持する理由はない。

## 決定

- selfhost compiler、生成された core module、生成された command component は、Rust 製
  host process を経由せず Wasmtime で直接実行する。
- リポジトリ独自の bridge（no repository-specific bridge）は存在しない。WASI の公式 Component Model 契約だけを実行境界とする。
- compiler の `--emit component` は core Wasm と WIT を出力し、command component の
  packaging は公式 `wasm-tools component embed/new` に委譲する。compiler 内に別の
  component encoder や互換 wrapping facade は持たない。packaging は
  `wasm-tools component new --reject-legacy-names` で旧 export 名を拒否する。
- WASI Preview 2 の component 出力は、公式 `wasi:cli`、`wasi:io`、および必要な公式
  WASI interface の import と canonical lowering で構成する。`arukellt:*`、独自の
  `runtime/host` import、埋め込み ABI bridge、実行時 adapter の plug は生成・実行経路に
  持たない。
- HTTP、TCP、stream、UDP の暫定 `std::host` API とその旧 ABI alias は削除する。互換
  wrapper は設けない。これらの機能が必要なコードは、公式 WASI interface を import する
  Component の境界で明示的に実装する。
- `host-linker`、WASI adapter/bridge、heap patcher、およびリポジトリの Cargo workspace
  は削除する。外部の `wasmtime`、`wasm-tools`、`wac` は生成・検証用ツールであり、
  リポジトリの runtime dependency には含めない。

## 却下した代替案

- 独自 bridge を残して公式 WASI import を名前だけ置き換える案は、Component Model の
  canonical ABI と実際の実行契約が分離するため採用しない。
- 旧 `std::host` API を deprecated alias として残す案は、削除対象の ABI と検証経路を
  再び温存するため採用しない。
- Rust host process を Wasmtime の薄い wrapper として残す案は、直接実行という決定を
  検証できず、Rust 依存を終わらせられないため採用しない。

## 帰結

この変更は HTTP/TCP/UDP/stream および独自 host ABI に対する破壊的な変更である。
利用者は公式 WASI Component の import 契約を選び、必要な host capability を実行環境へ
渡す。selfhost の bootstrap は直接実行できる Wasm を正本とし、実行時に Rust の
コンパイルや adapter の合成を要求しない。

公式 WASI interface の仕様変更、または直接 Wasmtime 実行で満たせない Component Model
の制約が明らかになった場合だけ、この判断を再検討する。

## 参照

- [ADR-000: ADR プロセスとステータスライフサイクル](ADR-000-process.md)
- [ADR-011: host-bound stdlib API は std::host:: に隔離する](ADR-011-wasi-host-layering.md)
- [ADR-034: Component 合成を wac plug に委譲](ADR-034-component-composition-linking.md)
- [ADR-053: セルフホストコンパイラ中核の再構築](ADR-053-selfhost-compiler-core-rewrite.md)
