# T1/T3 API 可用性の compiler / LSP / docs 統一表現

**Status**: done
**Created**: 2026-04-02
**Updated**: 2026-04-03
**ID**: 457
**Depends on**: 448, 455
**Track**: compiler, lsp, docs
**Blocks v1 exit**: yes
**Priority**: 1

---

## Closed by audit — 2026-04-03

**Reason**: All acceptance criteria verified by repo evidence.

**Evidence**: server.rs:1928-1945 hover shows T3 only warning, completion tags T3-only deprecated on T1

**Action**: Moved from `issues/open/` → `issues/done/` by false-done audit (confirmed truly-done).

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/457-t1-t3-availability-unified.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

`std::host::*` および capability-gated API について「補完には出るが hover で非対応表示」「compile 時は E0500 で止める」「代替 target を提案する」を compiler / LSP / docs で統一する。

現状は manifest の `target` フィールドがあっても LSP hover・completion・signature help への反映がない。`generate-docs.py` の `target_constraints` は manifest 非連動のハードコードである。E0500 diagnostic（Issue 448）が追加された後、その可否情報を LSP と docs が同じ manifest フィールドから読むようにする。

---

## 詳細実装内容

### Step 1: manifest の `availability` field を canonical source として確定する

Issue 455 で追加される `ManifestFunction.availability` を唯一の truth とする。

- `availability.t1: bool` — T1 (wasm32-wasi-p1 / Wasmtime core) で利用可能か
- `availability.t3: bool` — T3 (wasm32-wasi-p2 / component) で利用可能か
- `availability.note: Option<String>` — 補足（例: "T3 only via WASI Preview 2"）

本 issue では、Issue 455 が完了して manifest にこれらが存在することを前提とする。未完了の場合、`target` フィールド（"wasm32-wasi-p2" のみ = T3 only と推論）をフォールバックとして使う。

### Step 2: LSP hover に availability を表示する (`crates/ark-lsp/src/server.rs`)

`stdlib_hover_info` 関数（line ~1619）に以下のロジックを追加する。

```rust
fn format_availability(func: &ManifestFunction, current_target: Option<&TargetId>) -> Option<String> {
    let avail = func.availability.as_ref()?;
    let lines = vec![
        if avail.t1 && avail.t3 { "**Supported on:** all targets".to_string() }
        else if avail.t3 && !avail.t1 { "**Supported on:** wasm32-wasi-p2 (T3 only)".to_string() }
        else if avail.t1 && !avail.t3 { "**Supported on:** wasm32-wasi-p1 (T1 only)".to_string() }
        else { "**Supported on:** unavailable".to_string() }
    ];
    // If current target is set and incompatible, add warning
    if let Some(target) = current_target {
        let is_t1 = target == TargetId::WasmP1;
        let is_t3 = target == TargetId::WasmP2;
        if (is_t1 && !avail.t1) || (is_t3 && !avail.t3) {
            return Some(format!("⚠ **Not available on current target: {}**\n\n{}", target, lines.join("\n")));
        }
    }
    Some(lines.join("\n"))
}
```

hover Markdown の末尾に `---\n{availability_string}` を追加する。

### Step 3: LSP completion で target 非対応 symbol を降順に並べるか除外する

`textDocument/completion` で symbol リストを返す際に:

1. 現在の target（`InitializeParams` の `initializationOptions` または設定値から取得）と照合する。
2. 現在の target で使えない symbol は `CompletionItem.tags = [Deprecated]` を付けるか、`sortText` を後ろに送る。
3. 完全に除外はしない（ユーザーが意図的に使う場合もある）。

実装場所: `handle_completion` 関数（server.rs 内）。

### Step 4: signature help に availability 情報を追加する

`textDocument/signatureHelp` の `SignatureInformation.documentation` に `format_availability()` の結果を末尾に付加する。実装は hover と同じ関数を共有する。

### Step 5: generate-docs.py の `target_constraints` を manifest-driven に変更する

`scripts/gen/generate-docs.py` の各モジュール関数内のハードコードされた `"target_constraints"` を manifest の `availability` から生成する。

```python
def availability_string(functions: list[dict]) -> str:
    t1 = any(f.get("availability", {}).get("t1", True) for f in functions)
    t3 = any(f.get("availability", {}).get("t3", True) for f in functions)
    if t1 and t3:
        return "All targets."
    elif t3 and not t1:
        return "wasm32-wasi-p2 (T3) only. Requires WASI Preview 2 component model."
    elif t1 and not t3:
        return "wasm32-wasi-p1 (T1) only."
    else:
        return "No targets (stub)."
```

### Step 6: docs/stdlib 各 module page に availability badge を追加する

生成された module page（例: `docs/stdlib/io.md`）の関数テーブルに availability カラムを追加する。

```markdown
| Function | Signature | Stability | Targets |
|----------|-----------|-----------|---------|
| `get` | `get(url: String) -> Result<String, String>` | experimental | T3 only |
```

### Step 7: end-to-end 検証ケース

最低限以下の 2 関数でエンドツーエンドを通す:

1. `std::host::http::get` — T3 only
2. `std::host::sockets::connect` — T3 only

検証内容:

- manifest に `availability: { t1: false, t3: true }` が設定されている
- LSP hover が「Supported on: wasm32-wasi-p2 (T3 only)」を表示する
- `--target wasm32-wasi-p1` でのコンパイルが E0500 を出す（Issue 448 と統合）
- `docs/stdlib/modules/http.md` の availability が正しく表示される

---

## 依存関係

- Issue 448: E0500 診断コードが追加されていること
- Issue 455: `ManifestFunction.availability` が manifest schema に追加されていること

---

## 影響範囲

- `crates/ark-lsp/src/server.rs`（hover, completion, signature help）
- `scripts/gen/generate-docs.py`（target_constraints 生成ロジック）
- `docs/stdlib/` 生成結果（再生成で更新）
- `std/manifest.toml`（availability フィールドが必要）

---

## 後方互換性

- LSP hover に情報が追加されるだけ。壊れる既存挙動なし。
- docs の再生成は上書きのみ。

---

## 今回の範囲外

- T1/T3 以外のターゲット（将来追加分）
- capability フラグ（`--deny-clock` 等）のホバー表示（Issue 448 範囲）
- ユーザー定義関数への availability アノテーション

---

## 完了条件

- [x] `std::host::http::get` の hover に `⚠ Not available on wasm32-wasi-p1` が出る（target = p1 設定時）
- [x] `std::host::http::get` の hover に `Supported on: wasm32-wasi-p2 (T3 only)` が出る
- [x] completion リストで T1 非対応 symbol に deprecated tag or 後ろ送り sort が付く
- [x] `docs/stdlib/modules/http.md` に target availability カラムが存在する
- [x] `generate-docs.py` の `target_constraints` がハードコードではなく manifest から生成される
- [x] `bash scripts/run/verify-harness.sh` 通過

---

## 必要なテスト

1. LSP hover test: `http_get` を T1 target 設定で hover → "Not available on current target" が markdown に含まれる
2. LSP hover test: `http_get` を T3 target 設定で hover → availability string が正しい
3. `generate-docs.py` の unit test（`availability_string` 関数）
4. Issue 454 の LSP snapshot fixture に availability 表示を含める

---

## 実装時の注意点

- LSP サーバーは初期化時に `target` を受け取るが、設定変更時に動的に更新できるようにしておく（設定変更通知 `workspace/didChangeConfiguration` を処理する）。
- `availability` が manifest に未設定の場合（Issue 455 未完了関数）は availability 表示を省略し、既存の表示を維持する。
