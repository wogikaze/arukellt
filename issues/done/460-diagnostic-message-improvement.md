# エラーメッセージの文面と補助情報の統一整備

**Status**: open
**Created**: 2026-04-02
**Updated**: 2026-04-02
**ID**: 460
**Depends on**: none
**Track**: diagnostics
**Blocks v1 exit**: no
**Priority**: 2

## Summary

`unresolved name`・target 非対応・型エラーなど主要エラーコードの diagnostic 出力に「原因」「直し方」「関連箇所」が一貫して含まれるようにする。現状は `message` と `notes` にアドホックに情報が入っており、`help`（直し方）と `note`（補足情報）の区別がない。CLI と LSP で同じ文言系を使うことを保証し、代表的なエラーに snapshot test を追加する。

---

## 現状の問題

`Diagnostic` 構造体（`crates/ark-diagnostics/src/sink.rs`）には:
- `message: String` — エラーの主文（コードから自動設定）
- `notes: Vec<String>` — 複数の補足ノート
- `suggestion: Option<String>` — 1 件の提案
- `labels: Vec<Label>` — span に付くインラインラベル
- `fix_its: Vec<FixIt>` — 置換提案

しかし `help`（どう直すか）と `note`（背景情報）が `notes: Vec<String>` に混在している。Rust compiler の慣行では `note` と `help` は明確に別フィールドである。また `helpers.rs` の既存ヘルパー関数（`alias_warning_diagnostic` 等）は `with_note()` だけを使い、`with_suggestion()` はほぼ使われていない。

---

## 詳細実装内容

### Step 1: `Diagnostic` 構造体に `help` フィールドを追加する

```rust
pub struct Diagnostic {
    pub code: DiagnosticCode,
    pub message: String,
    pub phase_override: Option<DiagnosticPhase>,
    pub labels: Vec<Label>,
    pub fix_its: Vec<FixIt>,
    pub notes: Vec<String>,   // 補足情報（原因、関連背景）
    pub helps: Vec<String>,   // NEW: 直し方の提案（複数可）
    pub suggestion: Option<String>, // 既存: compat のため残す
}
```

`with_help(msg)` メソッドを追加する。既存の `with_suggestion()` は `with_help()` へのエイリアスとして残す（後方互換）。

### Step 2: `render.rs` と LSP 側で `helps` を別ラベルで出す

#### CLI レンダリング (`crates/ark-diagnostics/src/render.rs`)

```
error[E0100]: unresolved name `foo`
  --> src/main.ark:3:5
   |
 3 |     foo(42)
   |     ^^^ not found in this scope
   |
   = note: names must be declared before use with `let`, `fn`, `use`, or `import`
   = help: if `foo` is a function in another module, add `use <module>` at the top of the file
```

`note:` と `help:` を区別して出力する（`=` prefix でインデントを揃える）。

#### LSP レンダリング (`crates/ark-lsp/src/server.rs`)

LSP の `Diagnostic.message` は主文のみ。`relatedInformation` または `tags` で補足する。

`notes` → LSP `DiagnosticRelatedInformation` (if span is available)
`helps` → `message` の末尾に改行区切りで付加（`\n\nhelp: {text}`）

### Step 3: 主要エラーコードに `note` / `help` を追加する

#### E0100: unresolved name

```rust
pub fn unresolved_name_diagnostic(name: &str, span: Span, suggestions: &[&str]) -> Diagnostic {
    let mut d = Diagnostic::new(DiagnosticCode::E0100)
        .with_message(format!("unresolved name `{}`", name))
        .with_label(span, "not found in this scope")
        .with_note("names must be declared before use with `let`, `fn`, `use`, or `import`");
    if !suggestions.is_empty() {
        d = d.with_help(format!(
            "did you mean: {}?",
            suggestions.iter().map(|s| format!("`{}`", s)).collect::<Vec<_>>().join(", ")
        ));
    }
    d
}
```

呼び出し側（`crates/ark-resolve/src/lib.rs` 等）を新ヘルパーに移行する。

#### E0200: type mismatch

```rust
// 追加する note/help:
// note: expected `{expected}`, found `{actual}`
// help: consider using `as` for explicit conversion if types are compatible
```

#### E0300: missing field / undefined member

```rust
// note: `{type}` does not have a field named `{name}`
// help: available fields are: {field_list}
```

#### E0500: incompatible target (Issue 448 で追加予定)

```rust
// note: `{module}` requires WASI Preview 2 (wasm32-wasi-p2)
// help: use `--target wasm32-wasi-p2` to compile for WASI P2, or remove the import
// help: see also: https://arukellt.dev/docs/stdlib/modules/http.md
```

### Step 4: CLI と LSP の文言が同じことを保証する

`render_structured_snapshot()` (`render.rs`) が生成するテキストと、LSP が `Diagnostic.message` に入れるテキストが同じフォーマットから生成されることを保証する。

- 方針: `message_text()` / `note_texts()` / `help_texts()` を `Diagnostic` に getter として追加し、CLI も LSP も同じ getter を使う。
- helpers.rs の既存ヘルパーを新 builder メソッドに移行する。

### Step 5: 代表エラーコードの snapshot test を追加する

`crates/ark-diagnostics/src/render.rs` の test セクションに以下を追加する（各エラーにつき 1 件）。

対象エラー: E0100, E0200, E0300, E0400, E0401, E0402, W0001, W0002

```rust
#[test]
fn snapshot_e0100_unresolved_name() {
    let mut sm = SourceMap::new();
    let fid = sm.add_file("test.ark".into(), "fn main() { foo(42) }".into());
    let diag = unresolved_name_diagnostic("foo", Span::new(fid, 12, 15), &["foo_bar"]);
    let rendered = render_structured_snapshot(&diag, &sm);
    insta::assert_snapshot!(rendered);
    // または insta なしの場合:
    assert!(rendered.contains("unresolved name `foo`"));
    assert!(rendered.contains("note:"));
    assert!(rendered.contains("help:"));
    assert!(rendered.contains("did you mean: `foo_bar`?"));
}
```

`insta` crate が未使用なら `assert!` + `assert!(rendered.contains(...))` で代替する。

---

## 依存関係

- 依存なし（独立して着手可能）
- Issue 448 の E0500 が追加された後、E0500 にも同じパターンを適用する

---

## 影響範囲

- `crates/ark-diagnostics/src/sink.rs`（`helps` フィールド追加）
- `crates/ark-diagnostics/src/helpers.rs`（ヘルパー関数更新）
- `crates/ark-diagnostics/src/render.rs`（`help:` レンダリング追加、snapshot tests）
- `crates/ark-resolve/src/lib.rs`（E0100 ヘルパーへの移行）
- `crates/ark-lsp/src/server.rs`（LSP diagnostic 変換で `helps` を `message` 末尾に付加）

---

## 後方互換性

- `Diagnostic` 構造体への `helps` 追加は additive。既存の `with_suggestion()` をエイリアスとして残す。
- `render_structured_snapshot()` の出力形式が変わるため、既存 snapshot test は更新が必要。

---

## 今回の範囲外

- 全 diagnostic コードへの `note`/`help` 追加（主要 8 コードのみ）
- fix-it の自動適用（VSCode `quickFix` 連携）
- i18n / 多言語対応

---

## 完了条件

- [x] `Diagnostic` 構造体に `helps: Vec<String>` フィールドと `with_help()` メソッドが存在する
- [x] E0100 の diagnostic 出力に `note:` と `help:` の両方が含まれる
- [x] CLI の `arukellt check` と LSP diagnostics で同じ文言が出る（同一 fixture で確認）
- [x] E0100, E0200, E0300 の snapshot test が `cargo test -p ark-diagnostics` で pass する
- [x] `bash scripts/run/verify-harness.sh --quick` 通過

---

## 必要なテスト

1. `render_structured_snapshot` の snapshot test: E0100 / E0200 / E0300 / E0400 / W0001
2. `note:` と `help:` が別々のプレフィックスで出力されることの assert
3. suggestions が空の場合は `help:` が出ないことの assert
4. LSP diagnostic から `helps` が `message` 末尾に付加されることの integration test

---

## 実装時の注意点

- `helps: Vec<String>` は `notes` の後に並べる（表示順序: message → labels → notes → helps → fix-its）。
- 既存テストが `render_structured_snapshot` の出力を文字列比較している場合、`note:` ラベルの有無で差異が出る。既存テストの期待値を更新する。
- LSP の `DiagnosticRelatedInformation` は `location` が必須なので、span がない `help` は `message` 末尾への付加で対応する。
