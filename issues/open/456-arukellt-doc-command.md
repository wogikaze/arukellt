# `arukellt doc` サブコマンドの新設

**Status**: open
**Created**: 2026-04-02
**Updated**: 2026-04-02
**ID**: 456
**Depends on**: 455
**Track**: cli
**Blocks v1 exit**: no
**Priority**: 3

## Summary

`arukellt doc println` / `arukellt doc std::host::http::get` で CLI から標準ライブラリの説明・シグネチャ・target 可用性・stability・代替候補を引けるようにする `doc` サブコマンドを新設する。

現在の `Commands` enum には `doc` が存在しない。manifest から情報を引いてターミナルに整形出力し、`--json` で機械可読出力、`--target` で target-aware フィルタリング、見つからない時は近い候補提示を行う。

---

## 前提

- Issue 455 が完了し `ManifestFunction.doc` / `examples` / `errors` / `availability` が manifest に存在すること。
- Issue 455 完了前に本 issue に着手する場合は、`doc` フィールドなしでシグネチャ + stability + target のみ表示する最小実装から始め、Issue 455 完了後に doc/examples/errors を追加する。

---

## 詳細実装内容

### Step 1: `Commands` enum に `Doc` variant を追加 (`crates/arukellt/src/main.rs`)

```rust
/// Look up standard library documentation for a symbol or module
Doc {
    /// Symbol or module to look up (e.g. "println", "std::host::http::get", "std::host::http")
    symbol: String,
    /// Output as JSON
    #[arg(long)]
    json: bool,
    /// Filter availability for a specific target
    #[arg(long, value_name = "TARGET")]
    target: Option<TargetId>,
    /// Show all matching candidates even if an exact match exists
    #[arg(long)]
    all: bool,
},
```

`Commands::Doc` の処理ルーティングを `main.rs` の match に追加する。

### Step 2: `commands.rs` に `cmd_doc` 関数を実装する

新ファイル `crates/arukellt/src/cmd_doc.rs` を作成し、`mod cmd_doc;` で参照する。

#### 2.1: manifest のロード

```rust
pub fn cmd_doc(symbol: &str, json: bool, target: Option<&TargetId>, all: bool) -> bool {
    let manifest = match load_stdlib_manifest() {
        Ok(m) => m,
        Err(e) => { eprintln!("error: could not load stdlib manifest: {}", e); return false; }
    };
    // ...
}
```

`load_stdlib_manifest()` は `StdlibManifest::load_from_repo` を呼ぶ。repo root の探索は既存の LSP 初期化コードを参照する（`std::env::current_dir` → 親ディレクトリを遡る方法または `CARGO_MANIFEST_DIR` 相当）。

#### 2.2: シンボル解決

入力 `symbol` を以下の順で解決する。

1. **完全修飾関数名** `std::host::http::get` → module `std::host::http` + 関数名 `http_get`（マングル名の逆引き）
2. **短縮名** `get` → prelude または module から探す
3. **モジュール名** `std::host::http` → モジュール情報を表示
4. **大文字始まり** `String` → type 情報を表示
5. **見つからない場合** → fuzzy match で近い候補を返す

マングル名の逆引きロジック:
- `manifest.functions` の `name` フィールドは `http_get` のようにアンダースコア区切り。
- ユーザー入力 `std::host::http::get` を `http_get` に変換するルール: `std::host::X::Y` → `X_Y`（末尾の module::function）。
- このルールを `pub fn user_symbol_to_manifest_name(input: &str) -> String` として実装する。

#### 2.3: テキスト出力フォーマット

```
fn get(url: String) -> Result<String, String>
  Module: std::host::http
  Stability: experimental
  Supported on: wasm32-wasi-p2 (T3 only)
  ⚠ Not available on wasm32-wasi-p1

  HTTP GET リクエストを送信し、レスポンスボディを返します。

  Errors:
    DNS 解決失敗は Err("dns: ..."), 接続拒否は Err("connection refused: ...")

  Example:
    let body = http::get("https://example.com")
    match body { Ok(s) => println(s), Err(e) => eprintln(e) }

  See also: http::request
```

フォーマット実装ガイドライン:
- 幅: ターミナル幅に合わせる（`terminal_size` crate があれば使用、なければ 80 固定）。
- ANSI カラー: `std::env::var("NO_COLOR")` が設定されていない場合のみ使う。bold, dim で強調。
- `target` が非空かつ current target 引数と不一致の場合は `⚠ Not available on <target>` を先頭に表示する。

#### 2.4: JSON 出力フォーマット (`--json`)

```json
{
  "kind": "function",
  "name": "http_get",
  "display_name": "http::get",
  "module": "std::host::http",
  "params": ["String"],
  "returns": "Result<String, String>",
  "stability": "experimental",
  "target": ["wasm32-wasi-p2"],
  "availability": { "t1": false, "t3": true, "note": "..." },
  "doc": "HTTP GET リクエストを送信し...",
  "errors": "DNS 解決失敗は...",
  "examples": [
    { "code": "...", "description": "..." }
  ],
  "see_also": "http_request",
  "deprecated_by": null
}
```

`serde_json` で `ManifestFunction` を直接 serialize するか、出力専用の struct を定義する。

#### 2.5: target-aware フィルタリング (`--target`)

`--target wasm32-wasi-p1` が指定された場合:
- 関数の `target` が `["wasm32-wasi-p2"]` のみなら、先頭に `⚠ This function is not available on wasm32-wasi-p1.` を表示し、代替がある場合は提示する。
- モジュール検索で T1 非対応モジュールは除外しない（情報表示はする）。

#### 2.6: fuzzy match（見つからない場合）

```rust
fn find_closest_symbols(query: &str, manifest: &StdlibManifest, limit: usize) -> Vec<String> {
    // 単純なレーベンシュタイン距離またはプレフィックスマッチ
    let mut candidates: Vec<(usize, String)> = manifest
        .functions
        .iter()
        .filter(|f| f.kind.as_deref() != Some("intrinsic"))
        .map(|f| {
            let dist = levenshtein_distance(query, &f.name);
            (dist, f.name.clone())
        })
        .collect();
    candidates.sort_by_key(|(d, _)| *d);
    candidates.into_iter().take(limit).map(|(_, n)| n).collect()
}
```

レーベンシュタイン距離の実装は既存 crate (`strsim`) を使うか単純な実装を追加する。

出力例:
```
error: symbol 'httq_get' not found

Did you mean?
  http_get    (std::host::http)
  http_request  (std::host::http)
```

#### 2.7: モジュール表示

`symbol = "std::host::http"` の場合:
```
module std::host::http
  Stability: experimental
  Supported on: wasm32-wasi-p2

  HTTP client via WASI Preview 2.

  Functions:
    get(url: String) -> Result<String, String>  [experimental]
    request(method, url, body: String) -> Result<String, String>  [experimental]
```

### Step 3: `Cargo.toml` への依存追加

- `strsim` crate（fuzzy match 用）を `crates/arukellt/Cargo.toml` に追加する（または単純な levenshtein 実装をインライン実装）。
- `terminal_size` は optional（なければ 80 固定でも可）。

### Step 4: manifest の探索パス

`arukellt doc` を任意のディレクトリから実行できるようにするため、manifest の探索パスを以下の順で試みる。

1. `ARUKELLT_MANIFEST` 環境変数
2. 現在のディレクトリから上に向かって `std/manifest.toml` を探す
3. `~/.arukellt/stdlib/manifest.toml`（将来のインストール先）
4. コンパイル時に埋め込み（`include_str!` で最終フォールバック、ただし最新でない可能性あり）

現状の開発環境では 1 or 2 で十分。

### Step 5: ヘルプテキストの整備

`arukellt doc --help` 出力:

```
Look up standard library documentation

Usage: arukellt doc [OPTIONS] <SYMBOL>

Arguments:
  <SYMBOL>  Symbol or module to look up (e.g. "println", "std::host::http::get")

Options:
      --json              Output as JSON
      --target <TARGET>   Show availability for a specific target [possible values: wasm32-wasi-p1, wasm32-wasi-p2]
      --all               Show all matching candidates
  -h, --help              Print help
```

### Step 6: `arukellt targets` との統合確認

既存の `Commands::Targets` が存在する。`arukellt doc` と `arukellt targets` は独立しており、重複しない。

---

## 依存関係

- Issue 455 の `ManifestFunction.doc` / `examples` / `errors` / `availability` が存在することが前提。未完了の場合は最小実装（シグネチャ + stability + target のみ）で着手し、655 完了後に拡充する。
- Issue 457（target/capability 統一）は本 issue の `--target` フラグの表示と整合する。

---

## 影響範囲

- `crates/arukellt/src/main.rs`（`Commands::Doc` variant 追加）
- `crates/arukellt/src/cmd_doc.rs`（新規ファイル）
- `crates/arukellt/Cargo.toml`（`strsim` 等の依存追加）
- `docs/current-state.md`（`arukellt doc` コマンドの記載追加）

---

## 後方互換性・移行影響

- 新サブコマンドの追加は後方互換。既存コマンドへの影響なし。

---

## 今回の範囲外（明確な非対象）

- project symbols（ユーザー定義関数）の検索（将来拡張として `struct` に対応できる設計にしておくが実装しない）
- HTML / ブラウザでのドキュメント表示
- `arukellt doc --serve` での HTTP サーバー
- interactive fuzzy search (fzf 連携)

---

## 完了条件

- [ ] `arukellt doc println` がシグネチャ・module・stability を表示する
- [ ] `arukellt doc std::host::http::get` が target 制約を表示する
- [ ] `arukellt doc std::host::http` がモジュール内関数一覧を表示する
- [ ] `arukellt doc --json println` が valid JSON を返す
- [ ] `arukellt doc --target wasm32-wasi-p1 std::host::http::get` が「T1 非対応」を明示する
- [ ] 存在しないシンボルで `Did you mean?` 候補が出る
- [ ] `bash scripts/run/verify-harness.sh` が 13/13 pass

---

## 必要なテスト

1. `cargo test -p arukellt`: `cmd_doc` の unit test（manifest から正しくシンボルを解決する）
2. `user_symbol_to_manifest_name` 変換の unit test（`std::host::http::get` → `http_get` 等）
3. fuzzy match の unit test（存在しないシンボルで近い候補が返る）
4. `--json` 出力が valid JSON かつ期待フィールドを持つテスト
5. `--target wasm32-wasi-p1` で T3 専用関数に警告が出るテスト

---

## 実装時の注意点

- `user_symbol_to_manifest_name` は完全な双方向マッピングを持つのが理想だが、まず単方向（ユーザー入力 → manifest 名）を実装し、逆引きは manifest の `module` フィールドから `display_name` を再構築する。
- manifest に `doc` が設定されていない関数（Issue 455 が完了していない間）は `doc` 欄を省略する（空文字列でなく、フィールドを出さない）。
- `--json` の JSON スキーマは将来の LSP / IDE ツール連携を考慮した設計にする（`kind` フィールドで `"function"` / `"module"` / `"type"` を区別）。
