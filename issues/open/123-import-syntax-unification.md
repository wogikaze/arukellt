# import 構文と WIT パッケージ識別子の統一方針決定

**Status**: open
**Created**: 2026-03-28
**ID**: 123
**Track**: language-design
**Orchestration class**: design-ready
**Orchestration upstream**: —
**Blocks v4 exit**: yes
**ADR candidate**: yes

---

---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/123-import-syntax-unification.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## 問題定義

現在 Arukellt には **2種類の「モジュール参照」** が混在している。

| 種別 | 構文例 | 意味 | セパレータ |
|------|--------|------|-----------|
| ソースレベル stdlib import | `use std::io` | Arukellt ソース内でモジュールを参照する | `::` |
| WIT パッケージ識別子 | `wasi:cli/stdin@0.2.10` | Component Model のバイナリ境界で使われるパッケージ ID | `:` + `/` + `@` |

この2つは **現時点では別の場所に現れる** ため衝突しないが、v2 Component Model 対応で「ソースから外部 WIT インターフェースを参照する構文」が必要になった時点で、以下の問題が顕在化する。

1. LLM が `std::io` と `wasi:io/streams` を同じ層の概念として混同するリスク
2. Arukellt ソース内で WIT パッケージを参照する際の構文が未定義
3. `std::io` が WIT 上では `arukellt:std/io` に相当するのか、あるいは別物なのかが不明

---

## 調査資料

### 1. WIT 仕様の構文 (`docs/spec/spec-WASI-0.3.0-rc/`)

WIT の import/use 構文:

```wit
package wasi:clocks@0.2.10;      // パッケージ宣言: namespace:name@version

interface monotonic-clock {
    use wasi:io/poll@0.2.10.{pollable};  // 外部 interface 参照
}

world imports {
    import monotonic-clock;               // world 内 import
}
```

構造: `namespace:package-name/interface-name@semver.{symbol}`

- `:` はネームスペースとパッケージ名のセパレータ
- `/` はパッケージ名とインターフェース名のセパレータ
- `@` はバージョン
- `.{}` はシンボル列挙

### 2. Arukellt の現在の構文 (`crates/ark-parser/src/parser.rs`)

```
// lexer: TokenKind::ColonColon（::）でパス区切り
// TokenKind::Import → `import foo` (単一識別子)
// TokenKind::Use    → `use std::io::something` (:: 区切りパス)
```

2種類の import keyword が並存:
- `import math` — ローカルファイルベースのモジュール（単一識別子）
- `use std::io` — stdlib モジュール（`::` 区切りパス）

### 3. 他言語の事例

| 言語 | ソース内 import | WIT/Component 境界の処理 |
|------|----------------|--------------------------|
| Rust (cargo-component) | `use crate::...;` (`::`-区切り) | `wit-bindgen` が WIT から `::` パスの Rust コードを生成。ソースに WIT 構文は出てこない |
| Go (wasi) | `import "path/to/pkg"` (スラッシュ区切り) | WIT は外部ツールで処理。Go ソースは変更なし |
| Python (componentize-py) | `import module` (ピリオド区切り) | WIT は外部ファイルに分離 |
| MoonBit | `@namespace/package/module` | WIT との統合は外部ビルドツール層 |
| JavaScript (componentize-js) | ESM `import` | WIT は外部 `.wit` ファイル。JS ソースは変更なし |

**共通パターン**: 既存の言語はソース内 import 構文を WIT 識別子フォーマットに変更していない。WIT はビルドツール・バイナリ境界の概念として分離されている。

### 4. ADR-006（ABI 方針）の制約

ADR-006 は Layer 2A（raw Wasm ABI）と Layer 2B（WIT ABI）が「同じ言語セマンティクスから生成される」ことを要求する。これはソース構文と WIT 識別子を統一することを意味しない——むしろ **ソース側の意味論** と **WIT 境界の識別** は別レイヤーとして扱う方針である。

### 5. v2 Component Model 対応での要件 (issue #074)

Arukellt ソースが Component Model コンポーネントを生成する際に必要な宣言:

```
// A) 現在ないもの — コンポーネントが "export" する WIT world の宣言
// B) 現在ないもの — コンポーネントが "import" する外部 WIT インターフェースの宣言
// C) 既存 — 標準ライブラリの `use std::io` (stdlib へのアクセス)
```

A・B は **ソースファイルの外に持てる**（別 `.wit` ファイル + ビルドツール）か、**ソース内にアノテーションとして持つ** かを決める必要がある。

---

## 分析

### 問題の本質

LLM が混同するのは `std::io` と `wasi:io/streams` が「どちらも IO に関するモジュール参照」に見えるからであり、構文が似ているからではない。構文の違い（`::`vs`:`）は、むしろ **視覚的に2層を区別する手がかり** として機能しうる。

本当に解決すべきは:
1. **Layer の命名を明確にすること** — 「Arukellt stdlib import」と「Component Model パッケージ参照」は別の概念であることをドキュメントと言語仕様に明記する
2. **v2 以降で WIT 参照をどう扱うか** — ソース内アノテーション方式か、外部ファイル方式か

### 選択肢

#### 選択肢 A: 現状維持（2層を意図的に分離）

- `use std::io` → ソースレベル Arukellt import（変更なし）
- `wasi:cli/stdin` 参照 → 外部 `.wit` ファイルとビルドツール層で処理。ソース内に WIT 識別子は書かない
- 必要なら `#[wit_export("wasi:cli/run")]` のようなアノテーション構文を追加（v2）

**長所**:
- 既存コード全体が破壊ゼロ
- Rust/Go/Python と同じアプローチ（実績あり）
- `::` と `:` で視覚的に2層を区別できる
- セルフホスト時にコンパイラ自身が `use std::io` で書けて直感的

**短所**:
- WIT インターフェース識別子をソース内で書きたい場合（例: インラインでのコンポーネント宣言）に別構文が必要
- 2つの「import」概念が完全には一本化されない

#### 選択肢 B: `namespace:package/module` 形式を全面採用

- `use arukellt:std/io` → stdlib import
- `use wasi:cli/stdin` → WASI import
- ソースレベルの `::` を `:` + `/` に置き換える

**長所**:
- Component Model ネイティブに近い
- 統一された1フォーマット

**短所**:
- **全既存 fixture・stdlib の破壊的変更**（409件のテストが全部変わる）
- `arukellt:std/io` という書き方は冗長かつ不自然（Rust で `rust:std/io` と書かせるようなもの）
- stdlib 関数は `arukellt:std/io::writeln_stdout()` のような修飾が必要になり学習コストが増大
- セルフホスト時のコンパイラコードが読みにくくなる
- WIT の `namespace:package` は組織・レジストリの概念で、1つの言語の内部モジュールに使う設計ではない

#### 選択肢 C: `use`（`::` 区切り）を stdlib 専用に確定、`wit import` を別 keyword 化

- `use std::io` → 現行のまま（変更なし）
- `wit import "wasi:cli/stdin"` → Component Model 外部インターフェース参照の新構文（v2 で追加）

**長所**:
- 既存コード破壊ゼロ
- WIT 参照をソース内に書きたいユースケースに対応
- キーワードが異なるため LLM も区別しやすい

**短所**:
- `wit import` という新キーワードの追加コスト
- 現時点では実装不要（v2 以降の課題）

#### 選択肢 D: `import`（単一識別子）を廃止、`use`（`::` 区切り）に一本化

- `import math` → `use math`（または `use local::math`）に統一
- WIT 境界は外部ファイル方式のまま

**長所**:
- ソースレベルの2種類 import を1種類に削減
- LLM が `import` と `use` を混同しなくなる
- v2 に向けた準備として `import` keyword を将来の WIT 用に空けておける

**短所**:
- 既存の `import math` 構文を使っているコードへの影響（局所的、stdlib 外）
- `import` keyword を WIT 用途として予約するかどうかは別途決定が必要

---

## 推奨

**選択肢 A + 選択肢 D の組み合わせ**を推奨する。

1. **`use std::io` の `::` 形式はそのまま確定**（変更なし）
2. **`import 単識別子` を `use` に統一する**（v4 でのクリーンアップ）
3. **`import` keyword は Component Model / WIT 境界宣言用に予約する**（v4 で明文化）
4. **WIT パッケージ参照のソース内構文は v4 で `import "wasi:cli/stdin"` または `@[wit_import("wasi:cli/stdin")]` として設計**
5. ドキュメントに「`use` は Arukellt ソースの module 参照、`import` は外部コンポーネント参照（将来）」と明記

この方針により:
- 現行 409 件のテストへの影響: **ゼロ**
- LLM の混乱防止: ドキュメントと keyword の分離で対処
- v2/v4 の Component Model 対応: `import` keyword を空けることで対応可能
- セルフホスト適性: `use std::io` のままで読みやすい

---

## 実装タスク

### 即時（v3/v4 準備）

1. `docs/spec/language-reference.md`（または新規 `docs/spec/import-system.md`）に以下を明記:
   - `use std::io` — ソースレベルの Arukellt stdlib/module 参照
   - `import math` — ローカルファイルモジュール参照（`use` への統一候補）
   - WIT パッケージ識別子 (`wasi:cli`) — ソース内に出現しない、ビルドツール・バイナリ境界の概念
2. ADR-009 として本決定を記録する

### v4 以降

1. `import 単識別子` を `use 単識別子` に deprecate（W0101 警告）
2. `import "namespace:package/interface"` 構文（文字列リテラル形式）を Component Model 境界宣言として設計・実装
3. パーサーに `TokenKind::Import` を WIT import 専用 keyword として再割り当て

---

## 完了条件

- ADR-009 が `DECIDED` ステータスで存在する
- `docs/spec/import-system.md` が2層の構文を明記している
- `import 単識別子` の deprecate タイムラインが明文化されている
- 既存全 fixture が引き続き pass する（破壊的変更なし）

---

## 注意点

1. **選択肢 B（全面 `namespace:package` 採用）は採用しない**。WIT の `:` + `/` は組織・レジストリ識別子の設計であり、1言語の内部モジュール体系に適用するスケールではない
2. **即座の大規模リファクタは不要**。既存の `use std::io` を壊すメリットがない
3. `import` keyword の再利用は v4 で設計するが、v3 では現状の挙動を維持する

---

## 未解決論点

1. `import 単識別子`（ローカルファイルモジュール）を `use` に統合するタイミング: v4 deprecate → v5 除去 でよいか
2. Component Model WIT import をソース内で表現する際、文字列リテラル（`import "wasi:cli/stdin"`）か識別子形式（`import wasi:cli/stdin`）か
3. Arukellt 自身の WIT package ID を `arukellt:std/io` とするか、stdlib は WIT に公開しないと割り切るか

---

## 関連

- ADR-006: 公開 ABI 3層構造（Layer 2A/2B の分離根拠）
- ADR-007: コンパイルターゲット整理（T3 = wasm32-wasi-p2 が main target）
- issue #074: WASI p2 native component 対応
- `docs/spec/spec-WASI-0.3.0-rc/`: WIT 構文の一次資料

---

## Acceptance slice evidence — 2026-04-14 (docs/import-system contract)

This note records only the issue #123 docs acceptance slice for the import-system contract page.
It does not claim full issue closure.

- Added: `docs/spec/import-system.md` (Layer S vs Layer C contract; explicit current vs planned/deferred behavior)
- Linked from canonical ADR: `docs/adr/ADR-009-import-syntax.md` → `../spec/import-system.md`
- Repo-internal links resolve under docs consistency checks (including ADR-009 -> import-system contract page).
- Verification for this slice:
    - `bash scripts/run/verify-harness.sh --quick` (PASS)
    - `python3 scripts/check/check-docs-consistency.py` (PASS)

---

## Design artifact — ADR-025 draft (2026-04-16)

Partial advancement for issue #123 (documentation-only slice):

- Added: `docs/adr/ADR-025-use-paths-vs-wit-package-identifiers.md` (**Status: Proposed (draft)**)
  - Decision candidates (single WIT format vs two-layer default vs `wit` keyword vs WIT-outside-source)
  - Namespace / lexical collision avoidance
  - Non-binding Layer C syntax sketch (`import "ns:pkg/interface@ver"`)
  - Migration / compatibility table aligned with ADR-009
  - **Recommended default** in the draft: keep ADR-009 two-layer split (Candidate B)
- Cross-links: `docs/adr/ADR-009-import-syntax.md` (Related), `docs/spec/import-system.md` (§5), `docs/language/spec.md` (Appendix C), `docs/adr/README.md` (index table)

**Acceptance criteria still remaining for full issue closure** (see issue §完了条件 and §実装タスク):

1. Compiler/runtime: `import <single_identifier>` deprecation path (v4) and removal timeline (v5) as **implemented** behavior with diagnostics, not only docs.
2. Layer C: `import "…"` (or chosen form) **implemented** and wired to component / WIT pipeline (tracked with issue #124 and related work).
3. **完了条件** checklist: verify `docs/spec/import-system.md` and ADR-009 remain accurate vs `docs/current-state.md` after implementation lands; full fixture pass remains the bar for closing the issue, not this draft ADR alone.

Verification for this slice (to be recorded by agent):

- `python3 scripts/check/check-docs-consistency.py`
- `bash scripts/run/verify-harness.sh --quick` (repo skill; recommended)
