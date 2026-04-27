---
Status: open
Created: 2026-03-28
ID: 124
Track: language-design
Orchestration class: blocked-by-upstream
Orchestration upstream: None
Depends on: "074 (wasi-p2-native-component)"
Blocks v4 exit: yes
ADR candidate: yes
Implementation target: "Use Ark (src/compiler/*.ark) instead of Rust crates (crates/*) per #529 100% selfhost transition plan."
Status note: BLOCKED — downstream of the #074 WASI P2 native parent gate. Do not dispatch until #074 has P2 import-table, minimum Canonical ABI, and validate/run evidence.
---

# WIT component import syntax
package namespace: tui@0.1.0;
title: string,
body: string,
headings: list<string>,
parse-markdown: "func(input: string) -> document;"
parse-heading: "func(line: string) -> option<string>;"
width: u32,
height: u32,
clear-screen: "func();"
render-text: "func(text: string, x: u32, y: u32);"
get-frame-size: "func() -> frame;"
// セマンティクス: ""namespace:markdown-parser/parse" インターフェースの"
import "namespace: markdown-parser/parse" as md
use std: ":host::fs"
let content = fs: ":read_to_string("README.md")"
// md: ":Document は WIT の `record document { title: string, ... }` から生成"
let doc: "md::Document = md::parse_markdown(content)"
let frame: "tui::Frame = tui::get_frame_size()"
tui: ":render_text(doc.body, 0, 2)"
import_stmt: ":= "import" string_literal "as" ident"
string_literal: ":= '"' namespace ':' package '/' interface ('@' version)? '"'"
import "wasi: filesystem/types" as fs_types
"namespace: tui/render",
# "wasi: http" = { path = "vendor/wasi-http", version = "0.2.10" }
"wasi: "cli/run",      # main() → wasi:cli/run の export"
### Phase 1: "CLI `--wit` フラグ (最小実装、約1-2週)"
Stdlib,               // use std: ":host::stdio  (既存)"
Wit { package_id: "String },  // import "namespace:pkg/iface" (新規)"
1-3. ark-resolve: WIT import を型/関数として登録
fn register_wit_imports(doc: "&WitDocument, scope: &mut Scope) { ... }"
1-4. ark-typecheck: WIT import 関数の型検査
WIT からの関数呼び出し `md: ":parse_markdown(s)` に型チェックを適用。"
1-5. MIR lower: "WIT import 関数呼び出しを MirStmt::WitCall に変換"
dest: Option<Place>,
interface: "String,   // "namespace:markdown-parser/parse""
func: "String,        // "parse-markdown" (WIT ケバブケース名)"
args: Vec<Operand>,
1-6. ark-wasm T3 emitter: WitCall → Wasm import call
T3 emitter で `MirStmt: ":WitCall` を受け取り、Wasm の import call に変換。"
import section に `(import "namespace: "markdown-parser/parse" "parse-markdown" (func ...))` を追加。"
│       ├── mod.wit          # interface simple-host { add: "func(a: s32, b: s32) -> s32; }"
### Phase 2: "型バインディング生成 (struct/enum)"
WIT の `record document { title: string, ... }` を
`struct Document { title: String, ... }` として型システムに登録。
### Phase 3: `ark.toml` プロジェクトマニフェスト
pub package: PackageMetadata,
pub dependencies: HashMap<String, Dependency>,
pub world: Option<WorldConfig>,
### Phase 4: ドキュメント・cookbook
- `use std: ":host::stdio` (stdlib) と `import "wasi:cli/stdin"` (WIT) の区別を明記"
import "test: calculator/math" as calc
let result = calc: ":add(10, 32)"
let product = calc: ":multiply(6, 7)"
package test: calculator@0.1.0;
add: "func(a: s32, b: s32) -> s32;"
multiply: "func(a: s32, b: s32) -> s32;"
- issue #123: import 構文と WIT パッケージ識別子の統一方針
- issue #074: WASI p2 native component 対応
- issue #121: WASI p2 canonical ABI hardening
- ADR-006: 公開 ABI 3層構造
- ADR-008: "component wrapping (wasm-tools 依存)"
- `crates/ark-wasm/src/component/wit_parse.rs`: 既存 WIT パーサー
---
# WIT コンポーネント import — ソース構文・ark.toml・型バインディング生成



---

---

## Reopened by audit — 2026-04-03


**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/124-wit-component-import-syntax.md` — incorrect directory for an open issue.


## 概要

他言語で作られた Wasm コンポーネントの WIT インターフェースを Arukellt ソースから参照できるようにする。

現在 `crates/ark-wasm/src/component/wit_parse.rs` に WIT テキストパーサと `wit_interface_to_mir_imports()` が存在するが、ソース構文・CLI 接続・型バインディング生成がなく、実際には使えない状態である。

---

## ユースケース: TUI markdown エディタ

以下の構成で Arukellt から外部コンポーネントを使う場合を基準とする：

```
markdown-editor/
├── ark.toml               # プロジェクト/パッケージマニフェスト
├── vendor/
│   ├── namespace-markdown-parser/
│   │   ├── mod.wasm       # 他言語ビルド済みコンポーネント (Rust/C/Go/etc.)
│   │   └── mod.wit        # このコンポーネントのインターフェース定義
│   └── namespace-tui/
│       ├── mod.wasm
│       └── mod.wit
├── src/
│   └── main.ark           # Arukellt ソース
└── target/
    ├── main.wasm          # arukellt が生成するコアWasm
    ├── main.wit           # arukellt が自動生成するWIT
    └── main.component.wasm  # wasm-tools compose 後の最終コンポーネント
```

---

## WIT ファイル例 (vendor/)

### `vendor/namespace-markdown-parser/mod.wit`

```wit
package namespace:markdown-parser@0.1.0;

interface parse {
    record document {
        title: string,
        body: string,
        headings: list<string>,
    }

    parse-markdown: func(input: string) -> document;
    parse-heading: func(line: string) -> option<string>;
}
```

### `vendor/namespace-tui/mod.wit`

```wit
package namespace:tui@0.1.0;

interface render {
    record frame {
        width: u32,
        height: u32,
    }

    clear-screen: func();
    render-text: func(text: string, x: u32, y: u32);
    get-frame-size: func() -> frame;
}
```

---

## Arukellt ソース構文 (設計)

### `src/main.ark` (目標)

```ark
// 外部 WIT コンポーネントの import
// セマンティクス: "namespace:markdown-parser/parse" インターフェースの
//                関数と型を `md` という名前空間で使えるようにする
import "namespace:markdown-parser/parse" as md
import "namespace:tui/render" as tui
use std::host::fs

fn main() {
    let content = fs::read_to_string("README.md")

    // WIT record は Arukellt の struct として自動バインドされる
    // md::Document は WIT の `record document { title: string, ... }` から生成
    let doc: md::Document = md::parse_markdown(content)

    let frame: tui::Frame = tui::get_frame_size()
    tui::clear_screen()
    tui::render_text(doc.title, 0, 0)
    tui::render_text(doc.body, 0, 2)
}
```

### import 構文の文法

```
import_stmt ::= "import" string_literal "as" ident
              | "import" string_literal        // alias = 最後のセグメント名

string_literal ::= '"' namespace ':' package '/' interface ('@' version)? '"'
```

例:

```ark
import "namespace:markdown-parser/parse" as md
import "wasi:cli/stdin"            // alias = stdin (最後のセグメント)
import "wasi:filesystem/types" as fs_types
```

### 型マッピング (WIT → Arukellt)

| WIT 型 | Arukellt バインド型 | 備考 |
|--------|-------------------|------|
| `string` | `String` | |
| `u32`, `s32` | `i32` | Arukellt は符号付き i32 のみ |
| `u64`, `s64` | `i64` | |
| `f32` | `f32` | |
| `f64` | `f64` | |
| `bool` | `bool` | |
| `list<T>` | `Vec<T>` | |
| `option<T>` | `Option<T>` | |
| `result<T, E>` | `Result<T, E>` | |
| `record foo { ... }` | `struct Foo { ... }` | UpperCamelCase に変換 |
| `enum foo { a, b }` | `enum Foo { A, B }` | |
| `variant foo { ... }` | `enum Foo { ... }` | |
| `resource` | `i32` (handle) | v4 以降で proper resource type |
| `tuple<T, U>` | `(T, U)` | |

WIT のケバブケース名 (`parse-markdown`) → Arukellt のスネークケース名 (`parse_markdown`)

---

## `ark.toml` プロジェクトマニフェスト (設計)

```toml
[package]
name = "markdown-editor"
version = "0.1.0"
target = "wasm32-wasi-p2"

# 外部コンポーネント依存関係
# キー = WIT パッケージ ID, 値 = .wasm + .wit の場所
[dependencies]
"namespace:markdown-parser" = { path = "vendor/namespace-markdown-parser" }
"namespace:tui"             = { path = "vendor/namespace-tui" }
# バージョン付き:
# "wasi:http" = { path = "vendor/wasi-http", version = "0.2.10" }

# このコンポーネントの WIT world 宣言
[world]
name = "markdown-editor"
imports = [
    "namespace:markdown-parser/parse",
    "namespace:tui/render",
    "wasi:cli/stdin",
    "wasi:filesystem/preopens",
]
exports = [
    "wasi:cli/run",      # main() → wasi:cli/run の export
]
```

---

## 現在のインフラ状況

既存の実装（使えるもの）：

| コンポーネント | 場所 | 状態 |
|--------------|------|------|
| WIT テキストパーサ | `crates/ark-wasm/src/component/wit_parse.rs` | ✅ 動作中 |
| `wit_interface_to_mir_imports()` | 同上 | ✅ 動作中 |
| `MirImport` 構造体 | `crates/ark-mir/src/mir.rs` | ✅ 定義済み |
| WIT 生成 (MIR→WIT) | `crates/ark-wasm/src/component/wit.rs` | ✅ 動作中 |
| コンポーネント wrap | `crates/ark-wasm/src/component/wrap.rs` | ✅ wasm-tools 依存 |

不足しているもの：

| 必要なもの | 影響範囲 |
|-----------|---------|
| `import "..."` ソース構文 | パーサー、AST、リゾルバー |
| WIT ファイル読み込み (CLI + ark.toml) | ark-driver, arukellt main.rs |
| WIT → Arukellt 型バインディング生成 | ark-resolve, ark-typecheck |
| 関数の MIR import への接続 | ark-mir lower |
| Wasm export への `as` alias 解決 | ark-wasm emit |
| `ark.toml` パーサー | 新クレート or ark-driver |

---

## 実装タスク

### Phase 1: CLI `--wit` フラグ (最小実装、約1-2週)

`ark.toml` なしで単体 WIT ファイルを指定できる最小実装。

**1-1. パーサーに `import "..."` 構文を追加**

```rust
// crates/ark-parser/src/ast.rs に追加
pub enum ImportKind {
    Local,                // import math  (既存)
    Stdlib,               // use std::host::stdio  (既存)
    Wit { package_id: String },  // import "namespace:pkg/iface" (新規)
}
```

**1-2. CLI フラグ追加**

```bash
arukellt run --wit vendor/namespace-markdown-parser/mod.wit src/main.ark
arukellt build --target wasm32-wasi-p2 \
               --wit vendor/namespace-markdown-parser/mod.wit \
               --wit vendor/namespace-tui/mod.wit \
               src/main.ark
```

`--wit` は複数指定可。各 WIT ファイルのパッケージ ID (`package` 宣言) から自動的に名前空間を決定。

**1-3. ark-resolve: WIT import を型/関数として登録**

```rust
// ark-resolve/src/bind.rs
// WIT document を読み込み、interface ごとに名前空間を作成
// record → struct 型定義
// func → 外部関数宣言
fn register_wit_imports(doc: &WitDocument, scope: &mut Scope) { ... }
```

**1-4. ark-typecheck: WIT import 関数の型検査**

WIT からの関数呼び出し `md::parse_markdown(s)` に型チェックを適用。
WIT 型 → Arukellt 型の変換テーブルを `checker.rs` に追加。

**1-5. MIR lower: WIT import 関数呼び出しを MirStmt::WitCall に変換**

```rust
// ark-mir/src/mir.rs
pub enum MirStmt {
    // ... 既存
    WitCall {
        dest: Option<Place>,
        interface: String,   // "namespace:markdown-parser/parse"
        func: String,        // "parse-markdown" (WIT ケバブケース名)
        args: Vec<Operand>,
    },
}
```

**1-6. ark-wasm T3 emitter: WitCall → Wasm import call**

T3 emitter で `MirStmt::WitCall` を受け取り、Wasm の import call に変換。
import section に `(import "namespace:markdown-parser/parse" "parse-markdown" (func ...))` を追加。

**1-7. フィクスチャテスト**

```
tests/fixtures/wit_import/
├── vendor/
│   └── simple-host/
│       ├── mod.wit          # interface simple-host { add: func(a: s32, b: s32) -> s32; }
│       └── mod.wasm         # 実装 (別ツールでビルド or Rust で事前ビルド)
├── main.ark
├── main.expected
└── README.md
```

### Phase 2: 型バインディング生成 (struct/enum)

**2-1. WIT record → Arukellt struct**

WIT の `record document { title: string, ... }` を
`struct Document { title: String, ... }` として型システムに登録。

**2-2. WIT enum/variant → Arukellt enum**

WIT の `enum direction { north, south }` を
`enum Direction { North, South }` として型システムに登録。

**2-3. フィールドアクセスの型検査**

`doc.title` が `String` 型として推論されること。

### Phase 3: `ark.toml` プロジェクトマニフェスト

**3-1. `ark.toml` パーサー**

```rust
// crates/ark-driver/src/manifest.rs
pub struct ArkManifest {
    pub package: PackageMetadata,
    pub dependencies: HashMap<String, Dependency>,
    pub world: Option<WorldConfig>,
}
```

**3-2. `arukellt build` がカレントディレクトリの `ark.toml` を読む**

```bash
cd markdown-editor/
arukellt build  # ark.toml を自動検出、依存 WIT を読み込み
```

**3-3. world 宣言から WIT world 自動生成**

`ark.toml` の `[world]` セクションから、コンポーネントの WIT world テキストを生成する。

### Phase 4: ドキュメント・cookbook

**4-1. `docs/cookbook/component-import.md`**
- 外部コンポーネントを使う完全な手順（ark.toml + import + wasm-tools compose）
  
**4-2. `docs/spec/import-system.md`** (issue #123 より)
- `use std::host::stdio` (stdlib) と `import "wasi:cli/stdin"` (WIT) の区別を明記

---

## 完了条件

### Phase 1 完了条件

- `arukellt run --wit vendor/foo.wit main.ark` が動作する
- WIT インターフェースの関数が型チェックを通過する
- WIT 関数呼び出しを含む .ark が有効な Wasm component を生成する
- `tests/fixtures/wit_import/` のフィクスチャが pass する

### Phase 2 完了条件

- WIT record フィールドへのアクセスが型安全に動作する
- WIT enum の match が可能

### Phase 3 完了条件

- `arukellt build` が ark.toml を自動検出する
- `ark.toml` の dependencies から WIT ファイルを自動読み込みする

---

## 注意点

1. **WIT ケバブケースとArukelllt スネークケースの自動変換**は必須。`parse-markdown` → `parse_markdown`、`record document` → `struct Document`。変換規則を1箇所 (`wit_name_to_ark`) に集約し、パーサー・型チェッカー・emitter が共有する。

2. **u32/u64 問題**: Arukellt は `i32`/`i64` のみ。WIT の `u32`/`u64` を暗黙 cast するか型エラーにするかを決める必要がある。推奨: compile-time lint (W0200: "WIT u32 mapped to i32; values > 2^31-1 will wrap") で警告のみ。

3. **resource type は Phase 1 では opaque i32 handle として扱う**。proper resource lifecycle (drop, borrow) は v4 以降。Phase 1 で resource を返す関数は W0201 警告で暫定的に i32 として扱う。

4. **WIT `use` (cross-interface reference) は Phase 1 out-of-scope**。`use wasi:io/poll.{pollable}` のような構文は後回し。Phase 1 では単一インターフェースのみ対応。

---

## フィクスチャの具体例

### `tests/fixtures/wit_import/main.ark`

```ark
// --wit tests/fixtures/wit_import/vendor/calculator/mod.wit で起動

import "test:calculator/math" as calc

fn main() {
    let result = calc::add(10, 32)
    println(i32_to_string(result))   // → "42"

    let product = calc::multiply(6, 7)
    println(i32_to_string(product))  // → "42"
}
```

### `tests/fixtures/wit_import/vendor/calculator/mod.wit`

```wit
package test:calculator@0.1.0;

interface math {
    add: func(a: s32, b: s32) -> s32;
    multiply: func(a: s32, b: s32) -> s32;
}
```

### `tests/fixtures/wit_import/vendor/calculator/mod.wat` (or pre-built mod.wasm)

```wat
(module
  (func (export "add") (param i32 i32) (result i32)
    local.get 0
    local.get 1
    i32.add)
  (func (export "multiply") (param i32 i32) (result i32)
    local.get 0
    local.get 1
    i32.mul)
)
```

実際のテストでは wasmtime の `--invoke` か wasm-tools compose を使って結合検証する。

---

## Component Model での最終構成図

```
markdown-editor プロジェクトのビルドフロー:

src/main.ark
    ↓ arukellt build --target wasm32-wasi-p2
target/main.wasm             (core wasm, WIT import 宣言あり)
target/main.wit              (生成 WIT world)

vendor/namespace-markdown-parser/mod.wasm  (事前ビルド済み)
vendor/namespace-tui/mod.wasm              (事前ビルド済み)

    ↓ wasm-tools compose (または arukellt compose)
target/markdown-editor.component.wasm    (全コンポーネントを合成)

    ↓ wasmtime run
[実行]
```

`arukellt compose` コマンドの追加も視野に入れる（wasm-tools compose のラッパー）。

---

## 未解決論点

1. **`import "..."` vs `wit_import "..."` keyword**: issue #123 の推奨に従い `import` keyword を WIT import 専用にするが、既存の `import math`（ローカルファイル）との衝突をどう解消するか。選択肢: (a) `import "string" as alias` vs `import ident` で文法レベル分岐、(b) `import math` を `use ./math` に移行させ `import` を WIT 専用化、(c) 両方 `import` で文法上は同じだが resolver が分岐。**推奨: (a) — 文字列リテラルなら WIT、識別子ならローカル。**

2. **ark.toml の scope**: Phase 3 の `ark.toml` は単純な依存関係マップか、Cargo.toml 規模の機能を持つべきか。**v4 では最小限（name/version/dependencies/world のみ）でよい。**

3. **WIT `world` を Arukellt ソースに書かせるか**: 現時点では `ark.toml` の `[world]` セクションで宣言。ソース内 `@world(...)` アノテーションは v5（セルフホスト）まで不要。

4. **`wasm-tools compose` vs ネイティブ合成**: wasm-tools は外部ツールに依存する（現 ADR-008 と同様）。`arukellt compose` ラッパーを作るかどうか。

---

## 関連

- issue #123: import 構文と WIT パッケージ識別子の統一方針
- issue #074: WASI p2 native component 対応
- issue #121: WASI p2 canonical ABI hardening
- ADR-006: 公開 ABI 3層構造
- ADR-008: component wrapping (wasm-tools 依存)
- `crates/ark-wasm/src/component/wit_parse.rs`: 既存 WIT パーサー