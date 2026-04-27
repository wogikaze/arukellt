---
Status: done
Created: 2026-04-02
Updated: 2026-04-03
ID: 455
Track: stdlib-docs
Depends on: none
Orchestration class: implementation-ready
---
# stdlib metadata v2: manifest に doc/examples/errors/availability を追加し docs と LSP を拡充する
**Blocks v1 exit**: no
**Priority**: 2

## Summary

`std/manifest.toml` の `[[functions]]` エントリには現状 `name/params/returns/stability/doc_category/target/see_also/deprecated_by` フィールドが存在するが、**関数ごとの説明文 (`doc`)・コードサンプル (`examples`)・失敗条件 (`errors`)・T1/T3 可否の明示 (`availability`) が欠けている**。

これに加え、`scripts/gen/generate-docs.py` 内の `target_constraints` 文字列はモジュール単位でハードコードされており、manifest から自動生成されていない。LSP hover では `target` フィールドが表示されず、`stdlib_hover_info` は stability/deprecated/category しか出さない。

本 issue の目的は：

1. `ManifestFunction` に `doc`, `examples`, `errors`, `availability` フィールドを追加する。
2. `std/manifest.toml` の重要関数（`std::host::*` + prelude の主要関数）にこれらフィールドを埋める。
3. `generate-docs.py` のハードコード `target_constraints` を manifest の `target` / `availability` から自動生成に切り替える。
4. LSP hover に `doc` / target 制約を反映する。
5. `generate-docs.py` が `docs/stdlib/reference.md` と module pages に `doc`/`examples`/`errors` を自動反映する。

---

## 矛盾と前提

### 矛盾 1: `generate-docs.py` の `target_constraints` はハードコード

`scripts/gen/generate-docs.py` 行 107 等に `"target_constraints": "All targets. No host capability required."` という文字列がモジュール辞書にハードコードされている。一方 `ManifestFunction.target` フィールドはすでに存在し、`target = ["wasm32-wasi-p2"]` が http/sockets 関数に設定されている。

**採用方針**: `generate-docs.py` の `target_constraints` 生成をハードコード辞書から manifest の `target` フィールドへの自動変換に切り替える。マイグレーション期間中は辞書をフォールバックとして残し、manifest に `target` が設定されている場合は manifest 側を優先する。

### 矛盾 2: `ManifestFunction.doc` フィールドが Rust 側に未定義

`ManifestModule.doc` は存在するが `ManifestFunction` には `doc` フィールドがない（`crates/ark-stdlib/src/lib.rs` 確認済み）。TOML に `doc = "..."` を書いても現状は無視される。

**採用方針**: `ManifestFunction` に `pub doc: Option<String>` を追加する（Rust 側）。

---

## 詳細実装内容

### Step 1: `ManifestFunction` に新フィールド追加 (`crates/ark-stdlib/src/lib.rs`)

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct ManifestFunction {
    // ... 既存フィールド ...
    #[serde(default)]
    pub doc: Option<String>,                     // 追加: 関数の説明文（1–3 行）
    #[serde(default)]
    pub examples: Vec<ManifestExample>,          // 追加: コードサンプル
    #[serde(default)]
    pub errors: Option<String>,                  // 追加: 失敗条件の説明
    #[serde(default)]
    pub availability: Option<ManifestAvailability>,  // 追加: T1/T3 可否の明示
}

#[derive(Debug, Clone, Deserialize)]
pub struct ManifestExample {
    pub code: String,           // Ark コードスニペット
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub output: Option<String>, // 期待される出力（あれば）
}

#[derive(Debug, Clone, Deserialize)]
pub struct ManifestAvailability {
    pub t1: bool,   // wasm32-wasi-p1 で利用可能か
    pub t3: bool,   // wasm32-wasi-p2 で利用可能か
    #[serde(default)]
    pub note: Option<String>,  // 制約の補足説明
}
```

### Step 2: `std/manifest.toml` の priority 関数へのフィールド充填

以下のカテゴリを優先して `doc`, `examples`, `errors`, `availability` を充填する。

**Priority 1 — `std::host::*` 全関数**（capability 制約が重要）:
- `process::exit`, `process::abort` — doc + availability (t1: true, t3: true)
- `http::get`, `http::request` — doc + availability (t1: true, t3: true) + errors
- `sockets::connect` — doc + availability (t1: false, t3: true) + errors
- `fs::read_to_string`, `fs::write_string`, `fs::write_bytes` — doc + errors + availability
- `env::var` — doc + availability (t1: false, t3: true) + note

**Priority 2 — prelude 主要関数**（使用頻度高）:
- `println`, `print`, `eprintln` — doc + examples
- `concat`, `push`, `pop`, `len` — doc + examples
- `parse_i32`, `i32_to_string` — doc + examples + errors

**Priority 3 — `std::host::clock`, `std::host::random`**:
- 全関数 — doc + availability + capability note

#### TOML 記述例

```toml
[[functions]]
name = "http_get"
module = "std::host::http"
kind = "prelude_wrapper"
intrinsic = "__intrinsic_http_get"
params = ["String"]
returns = "Result<String, String>"
stability = "experimental"
target = ["wasm32-wasi-p2"]
doc_category = "http"
doc = "HTTP GET リクエストを送信し、レスポンスボディを返します。"
errors = "DNS 解決失敗は `Err(\"dns: ...\")`, 接続拒否は `Err(\"connection refused: ...\")`, HTTP 4xx/5xx は `Err(\"http N: ...\")`"
[functions.availability]
  t1 = true
  t3 = true
  note = "T1 は Wasmtime linker 経由。T3 native component path は将来拡張。"
[[functions.examples]]
  code = "let body = http::get(\"https://example.com\")\nmatch body { Ok(s) => println(s), Err(e) => eprintln(e) }"
  description = "URL から HTML を取得してコンソールに出力する"
```

### Step 3: `generate-docs.py` の `target_constraints` 自動生成化

`scripts/gen/generate-docs.py` の以下の箇所を修正する。

1. モジュール辞書の `target_constraints` ハードコード値を残しつつ、manifest の `ManifestFunction.target` と `ManifestModule.target` から自動生成するヘルパー関数 `build_target_constraints(module_name, functions)` を追加する。
2. `build_target_constraints` の出力形式: 全関数が同じ target → `"All targets."` / 一部関数のみ → `"wasm32-wasi-p2 only."` / モジュール単位で制約あり → `"Requires wasm32-wasi-p2."`
3. `render_stdlib_reference` と module page renderer で、`ManifestFunction.doc`, `errors`, `examples` を出力する。

```python
def build_target_constraints(module_name: str, funcs: list[dict]) -> str:
    targets = set()
    for f in funcs:
        t = f.get("target", [])
        if t:
            targets.update(t)
    if not targets or targets == {"wasm32-wasi-p1", "wasm32-wasi-p2"}:
        return "All targets. No host capability required."
    if targets == {"wasm32-wasi-p2"}:
        return "**wasm32-wasi-p2** required."
    return f"Targets: {', '.join(sorted(targets))}."
```

1. `FUNCTION_REQUIRED_FIELDS` から `doc_category` を必須のまま保ち、`doc` は optional とする（`check-docs-consistency.py` の validation rule は変えない）。
2. reference.md の各関数エントリに `doc` / `errors` を追加する列または説明行を加える。

### Step 4: LSP hover への target 制約・doc 反映 (`crates/ark-lsp/src/server.rs`)

`stdlib_hover_info` を拡張する。

```rust
fn stdlib_hover_info(name: &str, manifest: &StdlibManifest) -> Option<String> {
    let func = manifest.functions.iter().find(|f| f.name == name)?;
    // ... 既存 ...
    // doc を追加
    if let Some(ref doc) = func.doc {
        hover.push_str(&format!("\n\n{}", doc));
    }
    // target 制約を追加
    if !func.target.is_empty() {
        let targets = func.target.join(", ");
        hover.push_str(&format!("\n\n🎯 *Supported on:* `{}`", targets));
    }
    // availability の補足
    if let Some(ref avail) = func.availability {
        if !avail.t1 {
            hover.push_str("\n\n⚠️ *Not available on wasm32-wasi-p1*");
        }
        if let Some(ref note) = avail.note {
            hover.push_str(&format!("  \n{}", note));
        }
    }
    // errors を追加
    if let Some(ref errors) = func.errors {
        hover.push_str(&format!("\n\n**Errors:** {}", errors));
    }
    Some(hover)
}
```

### Step 5: `check-docs-consistency.py` 更新

`ManifestFunction` に新フィールドが追加されたため、consistency checker に以下を追加する。

1. `host_stub` または `kind = "host_stub"` な関数が `availability` を持つことを確認する（warning）。
2. `target` が `["wasm32-wasi-p2"]` な関数が `availability.t1 = false` を持つことを確認する（warning）。
3. `examples` を持つ関数のコードスニペットが空でないことを確認する（error）。

---

## 依存関係

- Issue 456（`arukellt doc` コマンド）は本 issue の `ManifestFunction.doc` / `examples` フィールドに依存する。本 issue を先に完了させること。
- Issue 457（T1/T3 availability 統一）は本 issue の `availability` フィールドの定義を再利用する。本 issue の schema を先に確定させること。
- Issue 458（CodeLens 再設計）は独立して進行可能。

---

## 影響範囲

- `crates/ark-stdlib/src/lib.rs`（`ManifestFunction` 構造体の拡張）
- `std/manifest.toml`（Priority 1–3 関数へのフィールド充填）
- `scripts/gen/generate-docs.py`（target_constraints 自動化、doc/examples/errors 出力）
- `scripts/check/check-docs-consistency.py`（新フィールドの validation rule 追加）
- `crates/ark-lsp/src/server.rs`（`stdlib_hover_info` の拡張）
- `docs/stdlib/reference.md`（再生成）
- `docs/stdlib/modules/` 以下の生成ページ（再生成）

---

## 後方互換性・移行影響

- `ManifestFunction` への optional フィールド追加は後方互換。未設定フィールドは `None` / `vec![]` になる。
- LSP hover の出力が変わる（target 制約・doc が追加される）。これは機能拡張であり、既存の hover テストが文字列の exact match をしている場合は更新が必要。

---

## 今回の範囲外（明確な非対象）

- project symbols（ユーザー定義関数）への `doc` / `examples` サポート
- manifest の JSON Schema 定義ファイルの生成
- `doc` フィールドのマークダウン rich テキスト（プレーンテキストのみ）
- `examples` の実行・CI 検証（Issue 457 スコープ）

---

## 完了条件

- [x] `ManifestFunction` に `doc`, `examples`, `errors`, `availability` フィールドが Rust 側で定義されている
- [x] `std::host::*` 全関数に `doc`, `availability` が設定されている
- [x] LSP hover で `http::get` 上に target 制約と doc が表示される
- [x] `generate-docs.py` の `target_constraints` が manifest から自動生成される（ハードコード辞書はフォールバックとして残す）
- [x] `docs/stdlib/reference.md` が再生成されて doc/errors を含む
- [x] `python3 scripts/check/check-docs-consistency.py` が 0 errors

---

## 必要なテスト

1. `crates/ark-stdlib/src/lib.rs` の unit test: `ManifestFunction` が新フィールドを正しく deserialize する
2. `crates/ark-lsp/tests/lsp_e2e.rs`: `http_get` hover に `"Supported on:"` または `"wasm32-wasi-p2"` が含まれる
3. `python3 scripts/check/check-docs-consistency.py --strict` 通過
4. `python3 scripts/gen/generate-docs.py --check` 通過

---

## 実装時の注意点

- `ManifestAvailability` の TOML 表現は `[functions.availability]` インライン table になる。TOML の配列内インライン table は `[[functions]]` の中で `[functions.availability]` と書けない点に注意。`availability = { t1 = true, t3 = true }` のインライン形式を使う。
- `examples` は `[[functions.examples]]` という sub-array になる。TOML の `[[array.of.tables]]` 構文が `[[functions]]` 内でネストできるか確認する。できない場合は `examples` を TOML 文字列の配列（コード文字列のみ）に単純化する。
- `generate-docs.py` の `build_target_constraints` 変更は既存の module page 出力を変える可能性がある。`--check` モードで差分を確認してから commit すること。