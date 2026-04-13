# ADR-009: Import 構文の決定 — ソースモジュール参照と Component Model 境界の分離

**Status**: DECIDED
**Date**: 2026-03-28
**Decided by**: Language-design track (issue #123)

## Context

Arukellt には 2 種類の「モジュール参照」が混在している。

| 種別 | 構文例 | セパレータ |
|------|--------|-----------|
| ソースレベル stdlib import | `use std::host::stdio` | `::` |
| WIT パッケージ識別子 | `wasi:cli/stdin@0.2.10` | `:` + `/` + `@` |

v2 Component Model 対応で「ソースから外部 WIT インターフェースを参照する構文」が必要になり、
以下の問題が顕在化した:

1. LLM が `std::host::stdio` と `wasi:io/streams` を同じ層の概念として混同するリスク
2. Arukellt ソース内で WIT パッケージを参照する際の構文が未定義
3. `import 単識別子` と `use パス` の 2 種類が並存し、初学者が迷う

### 他言語の事例

| 言語 | ソース内 import | WIT/Component 境界 |
|------|----------------|-------------------|
| Rust (cargo-component) | `use crate::...;` (`::`-区切り) | `wit-bindgen` がコード生成。ソースに WIT 構文は出ない |
| Go (wasi) | `import "path/to/pkg"` | WIT は外部ツールで処理 |
| Python (componentize-py) | `import module` | WIT は外部ファイルに分離 |
| JavaScript (componentize-js) | ESM `import` | WIT は外部 `.wit` ファイル |

**共通パターン**: 既存の言語はソース内 import 構文を WIT 識別子フォーマットに変更していない。
WIT はビルドツール・バイナリ境界の概念として分離されている。

## Decision

Normative import-system contract page: [../spec/import-system.md](../spec/import-system.md)

### 1. `use std::host::stdio` の `::`-separated 形式を Arukellt ソースモジュール参照として確定する

変更なし。Arukellt ソース内でモジュールを参照する際は `use path::to::module` 構文を使い続ける。

### 2. `import` keyword を Component Model / WIT 境界宣言用に予約する

`import 単識別子`（ローカルファイルモジュール）は v4 で `use` に統一し deprecate する。
空いた `import` keyword は v4 以降で WIT 外部インターフェース参照に使用する:

```
// v4 以降の構文案
import "wasi:cli/stdin@0.2.10"
```

### 3. 2 層を明確に命名する

- **Layer S (Source)**: `use` — Arukellt ソースの module 参照 (`::` 区切り)
- **Layer C (Component)**: `import` — 外部 Component Model インターフェース参照 (WIT 識別子)

## Rationale

1. **既存コードへの影響ゼロ** — 409 件のテスト fixture を一切変更しない
2. **主要言語の実績に準拠** — Rust, Go, Python, JS のすべてがソース import と Component 境界を分離
3. **LLM フレンドリ** — keyword が異なる (`use` vs `import`) ため、LLM が 2 層を区別しやすい
4. **セルフホスト適性** — コンパイラ自身が `use std::host::stdio` のような明示 import で書けて可読性が高い
5. **ADR-006 との整合** — Layer 2A (raw Wasm ABI) と Layer 2B (WIT ABI) の分離方針に合致

## Alternatives Considered

### A. `namespace:package/module` 形式を全面採用 (不採用)

- `use arukellt:std/io` のように WIT 識別子フォーマットに統一
- **却下理由**: 全既存 fixture の破壊的変更 (409 件)、`arukellt:std/io` は冗長、
  WIT の `namespace:package` は組織・レジストリの概念であり 1 言語の内部モジュールに使う設計ではない

### B. `wit import` 別 keyword 化 (部分採用)

- `wit import "wasi:cli/stdin"` のように新複合キーワードを追加
- **部分採用**: `import` 単体を WIT 用に再利用する方針を採用し、`wit` 修飾子は不要とした

## Implementation Timeline

| フェーズ | タスク |
|---------|--------|
| 即時 (v3) | `docs/spec/import-system.md` に 2 層の構文を明記 |
| v3 (issue #124) | `--wit <path>` CLI flag で WIT ファイルからの型バインディング生成 (Phase 1: stub/parse only) |
| v4 | `import 単識別子` を `use` に統一 (W0101 deprecation 警告) |
| v4 | `import "namespace:package/interface"` 構文の設計・実装 |
| v4 (issue #124) | WIT-imported 関数を ARK ソースから通常の `use` import と同じパターンで参照可能に |
| v5 | `import 単識別子` の完全除去 |

### WIT Component Import (Issue #124)

The `--wit <path>` CLI flag is already accepted by the compiler (`arukellt compile --wit <path>`).
Phase 1 (current): the flag validates that the WIT file exists and is only meaningful with
`--emit component` or `--emit all`. The actual WIT-to-binding generation is not yet implemented.

When fully wired, WIT imports will work as follows:

1. The user passes `--wit my_interface.wit` at compile time
2. The compiler reads the WIT file and generates type bindings
3. ARK source references WIT-imported functions via normal `use` syntax
4. The generated component embeds the correct WIT world and imports

## Consequences

- `use` は Arukellt ソースの module 参照として確定し、今後変更しない
- `import` keyword は v4 で WIT 境界宣言専用に再定義される
- パーサーの `TokenKind::Import` は v4 で WIT import 専用として再割り当てされる

## Related

- ADR-006: 公開 ABI 3 層構造 (Layer 2A/2B の分離根拠)
- ADR-007: コンパイルターゲット整理
- ADR-008: Component Model ラッピング戦略
- Issue #074: WASI p2 native component 対応
- Issue #077: WASI P2 HTTP (`std::host::http`)
- Issue #124: WIT component import syntax (`--wit` CLI flag)
- Issue #123: import 構文と WIT パッケージ識別子の統一方針決定
- Issue #139: WASI P2 sockets (`std::host::sockets`)
