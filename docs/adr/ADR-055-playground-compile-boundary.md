# ADR-055: Playground の compile 境界

ステータス: **ACCEPTED** — ブラウザは compiler と診断を提供し、ユーザープログラムを実行しない

決定日: 2026-09-12

関連: [ADR-017](ADR-017-playground-execution-model.md)、[ADR-032](ADR-032-playground-compiler-wasm-runner.md)、[ADR-054](ADR-054-host-linker-and-rust-runtime-retirement.md)

## 文脈

Playground には、core Wasm の import 名 `arukellt_io` を直接実装する TypeScript
runner が残っていた。
これは公式 WASI Component の実行環境ではなく、独自 ABI と stdin/stdout 契約を
ブラウザへ持ち込む bridge である。

一方、公式 WASI Preview 2 の component 実行には component packaging と公式 runtime
が必要であり、core Wasm をそのままブラウザで instantiate して置き換えられるものでは
ない。

## 決定

1. Playground の公開 API は parse、format、tokenize、typecheck、compile と診断に限定する。
2. `arukellt_io`、T2 runner、virtual stdin、compile-then-run API、およびそれらの UI は削除する。
3. compiler 自身をブラウザ内で動かすための最小 WASI P1 host は維持する。これは
   compiler host であり、ユーザープログラムの実行 bridge ではない。
4. ユーザープログラムを実行するときは、CLI が公式 `wasm-tools` で WASI P2 component
   を package し、Wasmtime または別の公式 Component runtime に渡す。
5. 旧 API への compatibility alias は設けない。ブラウザ実行を再導入する場合は、
   公式 WASI Component runtime とその capability 設定を対象に新しい ADR を採択する。

## 帰結

- Playground の Build は core Wasm を生成し、独自 host ABI を生成・実行しない。
- example catalog はソースを編集・compile-check するためのもので、ブラウザ内の実行
  出力や仮想 stdin を契約しない。
- 公式 P2 の実行検証は `scripts/run/arukellt-selfhost.sh` と component interop の
  wasm-tools / Wasmtime ゲートが owner になる。
