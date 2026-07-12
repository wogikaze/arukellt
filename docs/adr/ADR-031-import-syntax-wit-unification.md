# ADR-031: import 構文と WIT パッケージ識別子の統合

ステータス: **ACCEPTED** — 二層分離を確定。`use` は Layer S、`import` は Layer C に予約
日付: 2026-04-25
トラック: language-design
Issue: [#123](../../issues/done/123-import-syntax-unification.md)
廃止: [ADR-026](ADR-026-import-vs-wit-package-syntax.md)（統合）; [ADR-025](ADR-025-use-paths-vs-wit-package-identifiers.md) も廃止; [ADR-009](ADR-009-import-syntax.md) を精緻化
決定日: 2026-04-25

---

## 背景

Arukellt は現時点で、言語が Component Model 出力を目指すにつれ混同のリスクがある、構文的に異なる 2 つの「モジュール参照」表面を公開している:

| 表面 | 例 | 区切り | 出現箇所 |
|---------|---------|-----------|-----------------|
| **Layer S — ソース import** | `use std::io` | `::` | `.ark` ソースファイル |
| **Layer S — ローカルファイル import** | `import math` | （単一識別子） | `.ark` ソースファイル |
| **Layer C — WIT パッケージ識別子** | `wasi:cli/stdin@0.2.10` | `:` `/` `@` | `.wit` ファイル、CLI フラグ、マニフェスト |

これらの表面は、WIT テキストが `.wit` とツールにあり `use` パス内にないため、現時点では字句的に衝突しない。しかし Layer C 宣言がソースに近づくにつれ 3 つの問題が浮かぶ:

1. **概念的混乱** — LLM と新規コントリビュータが `std::io` と `wasi:io/streams` を同等概念と混同する。そうではない（`std::io` は Arukellt stdlib モジュールパス。`wasi:io/streams` は WebAssembly Component Model パッケージ識別子）。
2. **未定義の構文** — Arukellt ソースから外部 WIT インターフェースを参照するソースレベル構文がまだない（Component Model 出力に必要、issue #124）。
3. **2 つの `import` 表面** — `import math`（ローカルファイル）と `use std::io`（stdlib パス）が共存し、新規モジュールごとに「どちらのキーワード？」という疑問を生む。

---

## WIT パッケージ識別子構文（参考）

WebAssembly Component Model / WIT 仕様どおり:

```wit
package wasi:clocks@0.2.10;            // namespace:name@version

interface monotonic-clock {
}

world imports {
    import monotonic-clock;
}
```

構造: `namespace:package-name/interface-name@semver.{symbols}`

- `:` — 名前空間 / パッケージ区切り
- `/` — パッケージ / インターフェース区切り
- `@` — バージョン
- `.{}` — シンボル列挙

---

## 検討したオプション

### Option A — WIT パッケージ識別子構文を全面的に採用

すべての import について、Arukellt ソースの `::` パスを WIT の `namespace:package/module` 形式に置き換える。

<!-- skip-doc-check reason="legacy example not fixture-backed" owner="#683" kind="non-runnable" expires="2026-12-31" -->
```ark
use arukellt:std/io           // stdlib import (was: use std::io)
use wasi:cli/stdin            // WASI import
```

**欠点:**
- **既存 409 テストフィクスチャすべてへの破壊的変更** — すべての `use std::` パスを書き換え必須
- `arukellt:std/io` は冗長で長い（Rust で `rust:std/io` と書くのに似る）
- WIT の `namespace:package` は組織/レジストリ identity 向けで、言語内モジュールパス向けではない
- セルフホスティング可読性低下: コンパイラ自身は `use std::host::stdio` を使う。`arukellt:std/host/stdio` への切替は明瞭さを損なう
- 標準ライブラリ関数が `arukellt:std/io::writeln_stdout()` になる — 学習コストが高い

**判定**: **Rejected.** WIT 識別子形式は組織横断 component identity 向けに設計され、単一言語の標準ライブラリ参照向けではない。

### Option B — WIT ID にマップする Arukellt ネイティブ import 構文を定義（二層分離）

Layer S（ソースレベルモジュール import）には `use` + `::` を維持。WIT 識別子は Layer C 境界データとして、文字列、属性、または外部 `.wit` + CLI フラグで表現。

<!-- skip-doc-check reason="legacy example not fixture-backed" owner="#683" kind="non-runnable" expires="2026-12-31" -->
```ark
use std::io                   // Layer S -- stdlib module (unchanged)
use std::host::fs             // Layer S -- host-bound stdlib module (unchanged)
// Layer C expressed outside source:
//   --wit my_interface.wit   (CLI flag, already accepted)
//   #[wit_import("wasi:cli/stdin@0.2.10")]  (future attribute form)
```

**利点:**
- 既存コードへの破壊的変更ゼロ（409 フィクスチャ影響なし）
- Rust、Go、Python、JavaScript、MoonBit と同様（すべてソース import と WIT 境界処理を分離）
- 視覚的区別（`::` vs `:` + `/`）が読者とツールに異なる抽象レイヤーを示す
- セルフホスティング互換: Arukellt コンパイラは Arukellt で書かれ、読みやすいまま
- ADR-006 準拠: Layer 2A（生 Wasm ABI）と Layer 2B（WIT ABI）は設計上すでに分離。ソース構文がバイナリ形式を鏡映する必要はない

**欠点:**
- 2 概念（「Layer S」vs「Layer C」）のドキュメントが必要
- ソース内のインライン Component Model 宣言には依然構文決定が必要（将来フェーズに延期）

**判定**: **Chosen**（決定節参照）。

### Option C — `wit import` 専用複合キーワード

既存の `use`/`import` に加え、Layer C import 用の複合キーワードを導入:

<!-- skip-doc-check reason="legacy example not fixture-backed" owner="#683" kind="non-runnable" expires="2026-12-31" -->
```ark
use std::io                   // Layer S (unchanged)
wit import "wasi:cli/stdin"   // Layer C -- new compound keyword form
```

**利点:**
- 文法レベルでレイヤーの完全明示的曖昧さ解消
- LLM と IDE ツールが文を明確に分類できる

**欠点:**
- 新しい複合キーワード表面
- Option B の属性/文字列形式とほぼ重複 — 同じ問題を将来フェーズで解く

**判定**: 選択方向に部分的に統合。Option B の配信経路（`import "..."`）が単一キーワードで同じ曖昧さ解消を達成。

### Option D — ソースの `import`/`use` キーワードを統一し、`import` を Layer C に予約

`import <single-identifier>`（ローカルファイル import）を廃止して `use` に寄せ、`import` キーワードを Layer C（WIT）宣言に解放。

<!-- skip-doc-check reason="legacy example not fixture-backed" owner="#683" kind="non-runnable" expires="2026-12-31" -->
```ark
// 現行
import math           // local file module
use std::io           // stdlib module

// 次の移行（廃止予告）
use math              // W0101 warning: `import <id>` is deprecated; use `use <id>`
use std::io           // unchanged

import "wasi:cli/stdin@0.2.10"   // WIT package import via freed keyword
```

**利点:**
- 通常モジュールの「2 つの import キーワード」混乱を解消
- `import` キーワードが外部/component 境界を明示（異なるセマンティクス）
- `use` パスへの構造的破壊なし

**欠点:**
- パーサ変更と廃止診断（W0101）が必要
- `import <single-id>` 利用者は移行必須。影響はローカルファイルモジュール import に限定

**判定**: **Adopted as the migration path**（Option B と併用）。`import` キーワードは Layer C 表面用に予約。

---

## 決定

**選択方向: Option B + Option D の併用。**

1. **`use path::to::module` を Layer S ソース import 構文として確定** — 既存ソースファイルやフィクスチャへの変更なし。

2. **`import <single-identifier>` を次の移行で廃止、最終的に削除** — ローカルファイル import は `use <identifier>` に移行。廃止診断: W0101。

3. **`import` キーワードを Layer C（WIT/Component Model）用に予約** — 具体構文は TBD（現候補は文字列形式 `import "wasi:cli/stdin@0.2.10"`。issue #124 参照）。

4. **WIT パッケージ識別子は Layer C 境界データのまま** — `.wit`、CLI フラグ、マニフェストに現れる。`use` パスセグメントとしては**無効**。

5. **レイヤー命名は正規**:
   - **Layer S (Source)**: `use` + `::` 区切りパス
   - **Layer C (Component)**: `import` + WIT 識別子形式

---

## 根拠

WIT の `namespace:package/interface@version` 形式は、WebAssembly Component Model エコシステムにおける組織横断パッケージ identity 向けに設計された。stdlib 参照に適用すると `arukellt:std/io::writeln_stdout()` となり、モジュール位置ではなくレジストリ identity を伝える — 主要な Component Model 言語のどれとも不一致:

| 言語 | ソース import | WIT 境界 |
|----------|--------------|--------------|
| Rust (cargo-component) | `use crate::...` (`::`) | `wit-bindgen` がコード生成。WIT はソースに現れない |
| Go (WASI) | `import "path/to/pkg"` | WIT は外部ツール |
| Python (componentize-py) | `import module` | WIT は外部ファイル |
| JavaScript (componentize-js) | ESM `import` | WIT は外部 `.wit` ファイル |

いずれもソースレベル import 構文は不変で、WIT はバイナリ境界のツールチェーン課題として扱われる。Arukellt も同じ分離を採用する。

`import` キーワード統一（Option D）は、stdlib パスを変えずに初心者の「`import math` か `use math` か？」混乱を減らす狭い QoL 改善。

---

## 移行への影響

### 既存コード（現行）

**変更不要。** 既存のすべての `use std::...` と `import <local>` ソース構文は現行のコンパイル・診断パスを継続。

### 次の移行経路

| 構文 | 次の移行での挙動 | 影響範囲 |
|--------|-------------|----------------|
| `use std::io` | 不変、警告なし | すべての stdlib / host import |
| `use path::to::module` | 不変、警告なし | すべての `use` パス import |
| `import math` (local file) | W0101 廃止警告。依然コンパイル可 | ローカルファイルモジュール import のみ |
| `import "wasi:cli/stdin"` | 新 Layer C 構文（`--emit component` でゲート） | 新規コードのみ |

`import <single-id>` から `use` への移行の推定フィクスチャ影響: **局所的**。stdlib は全体で `use` を使用。ほとんどの `import <single-id>` は特定テストフィクスチャに現れる。

### 最終削除

`import <single-identifier>` パース経路を削除。残存はハードエラー。

---

## 結果

- `use` は安定した永続の Arukellt ソースモジュール import キーワード。再定義されない。
- `import` は Component Model / WIT 境界キーワードとなる。`import <single-id>` セマンティクスは移行対象。
- ツール（IDE、LLM プロンプト、ドキュメント）は `use` と `import` を同義語ではなく別レイヤーとして説明すべき。
- `std::` パスプレフィックスは Arukellt ソース名前空間であり、WIT 名前空間ではない。WIT identity 用語の `wasi:` や `arukellt:` と同等ではない。

---

## 関連

- [ADR-009-import-syntax.md](ADR-009-import-syntax.md) — 主要決定記録（ACCEPTED）。本 ADR は本文、完全なオプション表、明示的移行影響でその決定を統合・拡張。
- [ADR-025-use-paths-vs-wit-package-identifiers.md](ADR-025-use-paths-vs-wit-package-identifiers.md) — 衝突ポリシーと構文探索（本 ADR により SUPERSEDED）。
- [ADR-006-abi-policy.md](ADR-006-abi-policy.md) — ABI レイヤー。ソース構文が WIT テキストを鏡映する必要はない。
- [ADR-007-targets.md](ADR-007-targets.md) — プライマリターゲット `wasm32-gc`（旧 alias `wasm32-wasi-p2`）。
- [../spec/import-system.md](../spec/import-system.md) — 規範的 Layer S / Layer C 契約ページ。
- [../module-resolution.md](../module-resolution.md) — `use` / `import` の Layer S 解決挙動。
- Issue [#074](../../issues/open/074-wasi-p2-native-component.md) — WASI p2 ネイティブ component 出力。
- Issue [#124](../../issues/done/124-wit-component-import-syntax.md) — WIT component import 構文実装。
