# VSCode API を使った editor behavior E2E テストを追加する

**Status**: open
**Created**: 2026-04-02
**Updated**: 2026-04-13
**ID**: 453
**Depends on**: 450, 451, 452
**Track**: vscode-ide
**Blocks v1 exit**: no
**Priority**: 4

---


## Reopened by audit — 2026-04-13

**Reason**: E2E test suites skipped.

**Action**: Moved from issues/done/ to issues/open/ by false-done audit.

## Closed by audit — 2026-04-03

**Reason**: All acceptance criteria verified by repo evidence.

**Evidence**: extension.test.js:259 definition range test, :360 hover-string-literal test, :424 zero-diagnostics test; fixtures/basic.ark present

**Action**: Moved from `issues/open/` → `issues/done/` by false-done audit (confirmed truly-done).

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/453-vscode-e2e-editor-behavior-tests.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

現在の `extensions/arukellt-all-in-one/src/test/extension.test.js` は extension の activation と command 存在確認のみを行う。`textDocument/hover` / `textDocument/definition` / `textDocument/diagnostic` の正しさを VS Code API 経由で検証するテストがなく、Issue 450/451/452 のような regression が CI で検出できない。本 issue では `vscode.executeDefinitionProvider` / `vscode.executeHoverProvider` / `languages.getDiagnostics` を使った E2E テストを追加し、range/contents/diagnostics 件数まで assertion する。

---

## 現状

`extensions/arukellt-all-in-one/src/test/extension.test.js` の既存テスト:
- extension が見つかること
- `.ark` ファイルで activate すること
- activate / deactivate が export されること
- 無効バイナリパスが crash しないこと

これらは「動いているか」の確認であり、LSP の機能の正しさは検証できていない。

---

## 詳細実装内容

### Step 1: テスト fixture `.ark` ファイルの追加

`extensions/arukellt-all-in-one/src/test/fixtures/` 以下に以下のファイルを追加する。

**`basic.ark`**:

```ark
fn greet(name: String) -> String {
    let msg = concat("Hello, ", name)
    msg
}

fn main() {
    let result = greet("world")
    println(result)
}
```

このファイルを参照した E2E テストを書く。

### Step 2: Go to Definition E2E テスト追加

`extension.test.js` に以下のテストスイートを追加する。

```js
suite("Go to Definition (#450)", () => {
  let doc;
  suiteSetup(async () => {
    const fixturePath = path.join(__dirname, "fixtures", "basic.ark");
    doc = await vscode.workspace.openTextDocument(fixturePath);
    await vscode.window.showTextDocument(doc);
    // LSP が解析を完了するまで待つ
    await new Promise((r) => setTimeout(r, 3000));
  });

  test("local variable definition range is identifier only", async () => {
    // `result` の使用箇所（main 関数内の println(result) の result）
    // basic.ark の line 7 (0-indexed), col 13 (println( の後)
    const pos = new vscode.Position(7, 13);
    const locs = await vscode.commands.executeCommand(
      "vscode.executeDefinitionProvider",
      doc.uri,
      pos
    );
    assert.ok(locs && locs.length > 0, "Should find definition");
    const loc = locs[0];
    // definition は line 6 の `let result = ...` の `result` 部分のみを指す
    assert.strictEqual(loc.range.start.line, 6, "Should point to let line");
    assert.strictEqual(loc.range.start.character, 8, "Should start at 'result'");
    // range が 1 行に収まること（全 let 文にならないこと）
    assert.strictEqual(
      loc.range.start.line,
      loc.range.end.line,
      "Definition range should be single line (not full let statement)"
    );
    // range の長さが 'result' (6文字) 程度であること
    const rangeLen = loc.range.end.character - loc.range.start.character;
    assert.ok(rangeLen <= 10, `Range too wide: ${rangeLen} chars`);
  });

  test("function definition range is function name only", async () => {
    // `greet` の呼び出し箇所（line 6, col 17）
    const pos = new vscode.Position(6, 17);
    const locs = await vscode.commands.executeCommand(
      "vscode.executeDefinitionProvider",
      doc.uri,
      pos
    );
    assert.ok(locs && locs.length > 0, "Should find greet definition");
    const loc = locs[0];
    // fn greet(...) の `greet` は line 0, col 3
    assert.strictEqual(loc.range.start.line, 0);
    assert.strictEqual(loc.range.start.character, 3);
    const rangeLen = loc.range.end.character - loc.range.start.character;
    assert.ok(rangeLen <= 8, `greet range too wide: ${rangeLen}`);
  });

  test("definition on whitespace returns nothing", async () => {
    // 空白位置
    const pos = new vscode.Position(0, 0);
    const locs = await vscode.commands.executeCommand(
      "vscode.executeDefinitionProvider",
      doc.uri,
      pos
    );
    // fn キーワード上では definition なし（または null）
    assert.ok(!locs || locs.length === 0, "keyword/whitespace should return no definition");
  });
});
```

### Step 3: Hover E2E テスト追加

```js
suite("Hover (#451)", () => {
  let doc;
  suiteSetup(async () => {
    const fixturePath = path.join(__dirname, "fixtures", "basic.ark");
    doc = await vscode.workspace.openTextDocument(fixturePath);
    await vscode.window.showTextDocument(doc);
    await new Promise((r) => setTimeout(r, 3000));
  });

  test("string literal returns no hover", async () => {
    // basic.ark line 0 の "Hello, " 文字列リテラル上
    const pos = new vscode.Position(1, 25); // concat("Hello, ", name) の "Hello, " の中
    const hovers = await vscode.commands.executeCommand(
      "vscode.executeHoverProvider",
      doc.uri,
      pos
    );
    // 有意な hover がないこと（空または string literal ノイズがないこと）
    const hasNoise = hovers && hovers.some(
      (h) => h.contents.some(
        (c) => (typeof c === "string" ? c : c.value || "").includes("string literal")
      )
    );
    assert.ok(!hasNoise, "String literal should not produce 'string literal' hover noise");
  });

  test("known function produces meaningful hover", async () => {
    // println の呼び出し on line 7
    const pos = new vscode.Position(7, 4);
    const hovers = await vscode.commands.executeCommand(
      "vscode.executeHoverProvider",
      doc.uri,
      pos
    );
    assert.ok(hovers && hovers.length > 0, "println should produce hover");
    const content = hovers
      .flatMap((h) => h.contents)
      .map((c) => (typeof c === "string" ? c : c.value || ""))
      .join("\n");
    assert.ok(content.includes("println") || content.includes("fn"), 
      "hover should contain function signature");
  });
});
```

### Step 4: Diagnostics E2E テスト追加

```js
suite("Diagnostics (#452)", () => {
  test("valid ark file produces no diagnostics", async () => {
    const fixturePath = path.join(__dirname, "fixtures", "basic.ark");
    const doc = await vscode.workspace.openTextDocument(fixturePath);
    await vscode.window.showTextDocument(doc);
    await new Promise((r) => setTimeout(r, 4000)); // LSP 解析待ち

    const diags = vscode.languages.getDiagnostics(doc.uri);
    assert.strictEqual(
      diags.length, 0,
      `Valid file should have no diagnostics, got: ${diags.map((d) => d.message).join(", ")}`
    );
  });

  test("file with unresolved name produces E0100", async () => {
    const content = "fn main() {\n    println(undefined_var)\n}\n";
    const doc = await vscode.workspace.openTextDocument({
      language: "arukellt",
      content,
    });
    await vscode.window.showTextDocument(doc);
    await new Promise((r) => setTimeout(r, 4000));

    const diags = vscode.languages.getDiagnostics(doc.uri);
    const hasE0100 = diags.some((d) => d.message.includes("E0100") || d.message.includes("unresolved"));
    assert.ok(hasE0100, "Should have E0100 for undefined_var");
  });

  test("diagnostics are stable after file change", async () => {
    const doc = await vscode.workspace.openTextDocument({
      language: "arukellt",
      content: "fn main() { println(\"hello\") }\n",
    });
    await vscode.window.showTextDocument(doc);
    await new Promise((r) => setTimeout(r, 3000));

    const diags1 = vscode.languages.getDiagnostics(doc.uri);
    assert.strictEqual(diags1.length, 0, "Should have no errors initially");

    // ファイル内容変更（有効なまま）
    const edit = new vscode.WorkspaceEdit();
    edit.replace(
      doc.uri,
      new vscode.Range(0, 0, doc.lineCount, 0),
      "fn main() { println(\"world\") }\n"
    );
    await vscode.workspace.applyEdit(edit);
    await new Promise((r) => setTimeout(r, 3000));

    const diags2 = vscode.languages.getDiagnostics(doc.uri);
    assert.strictEqual(diags2.length, 0, "Should still have no errors after edit");
  });
});
```

### Step 5: テスト実行環境の確認

`extensions/arukellt-all-in-one/package.json` のテストスクリプトを確認し、上記テストが既存のテスト実行方法（`vscode-test` / `@vscode/test-electron`）で動くことを確認する。

新しいテストで使う `vscode.commands.executeCommand("vscode.executeDefinitionProvider", ...)` は VS Code E2E テスト環境でのみ動作する（`vscode` モジュールが必要）。既存の activation テストと同じファイルに追加すれば動作するはず。

### Step 6: CI での実行確認

`.github/workflows/` または `scripts/` にある CI 設定で extension テストが実行されているか確認する。実行されていない場合は、テストを CI に組み込む手順を記載する（本 issue のスコープ: 確認のみ。CI 組み込みは別 issue でも可）。

---

## 依存関係

- Issue 450（definition span 修正）完了後に definition range の assertion が正しく pass する。
- Issue 451（hover noise 除去）完了後に "string literal" hover assertion が正しく pass する。
- Issue 452（偽陽性除去）完了後に diagnostics assertion が正しく pass する。
- 上記 issue の完了を待たずに本 issue を作成・マージし、暫定的に failing テストとして入れることも可能。その場合は各テストに `// expected to fail until #450/#451/#452` コメントを付ける。

---

## 影響範囲

- `extensions/arukellt-all-in-one/src/test/extension.test.js`（テスト追加）
- `extensions/arukellt-all-in-one/src/test/fixtures/basic.ark`（新規 fixture）

---

## 後方互換性・移行影響

- テスト追加のみ。既存テストへの影響なし。

---

## 今回の範囲外（明確な非対象）

- hover の markdown 内容の exhaustive assertion（snapshot は Issue 454 スコープ）
- cross-file definition の E2E（マルチファイル resolve の前提）
- CI への完全な組み込み（確認のみ）

---

## 完了条件

- [x] `basic.ark` fixture が `extensions/arukellt-all-in-one/src/test/fixtures/` に存在する
- [x] definition range が identifier のみを指すことの E2E テストが追加されている
- [x] string literal 上で hover が出ないことの E2E テストが追加されている
- [x] 有効ファイルで diagnostics が 0 件であることの E2E テストが追加されている
- [x] すべての新規テストが `npm test`（extension test suite）で pass する（Issue 450/451/452 完了後）

---

## 必要なテスト

1. `test("local variable definition range is identifier only")` — Issue 450 の受け入れ確認
2. `test("function definition range is function name only")` — Issue 450 の受け入れ確認
3. `test("string literal returns no hover")` — Issue 451 の受け入れ確認
4. `test("known function produces meaningful hover")` — regression 防止
5. `test("valid ark file produces no diagnostics")` — Issue 452 の受け入れ確認
6. `test("diagnostics are stable after file change")` — Issue 452 のキャッシュ安定性確認

---

## 実装時の注意点

- `vscode.commands.executeCommand("vscode.executeDefinitionProvider", uri, position)` の結果は `Location[]` または `LocationLink[]`。両方の型を処理できるようにする（`loc.range` と `loc.targetRange` の両方を確認）。
- LSP の応答を待つために `setTimeout` を使っているが、より確実にするには `vscode.languages.registerDefinitionProvider` が登録されるのを待つか、LSP の `initialized` 通知を待つ仕組みが必要。既存の activation テストで使われている待機パターンに倣う。
- `fixtures/basic.ark` のコードは実際に `arukellt check` がエラーなしで通るコードにする。事前に CLI でチェックを通してから fixture として追加すること。
- line/column の 0-indexed と 1-indexed の混在に注意する。VS Code の `Position` は 0-indexed。
