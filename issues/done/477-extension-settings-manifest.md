---
Status: done
Created: 2026-04-03
Updated: 2026-04-03
ID: 477
Track: extension
Depends on: "(none)"
Orchestration class: implementation-ready
Blocks v1 exit: no
---

# Extension package.json: 5 arukellt 設定項目の宣言
- #478: "extension.js initializationOptions wiring (depends on this)"
- #479: "LSP server LspConfig + handler behavior (depends on #478)"
- #480: "README settings table (depends on #479)"
(`type: "boolean`, `default: true`, `description` 設定済み, `scope: resource`)"
- `extensions/arukellt-all-in-one/package.json: contributes.configuration.properties` の該当 5 行
→ これは意図通り: この issue は manifest 宣言のみを担当する
- LSP が設定を無視していても close できる → これは意図通り: LSP は #479 で担当
- `grep "check.onSave" extensions/arukellt-all-in-one/package.json` → 1 match (`"arukellt.check.onSave": {`)
# Extension package.json: 5 arukellt 設定項目の宣言

---

## Decomposed from 462

Issue 462 (`extension-settings-rationalization`) mixes 5 layers:
manifest declaration / extension wiring / LSP implementation / tests / docs.
This issue covers **manifest layer only** — declaring settings in `package.json`.

Downstream issues:
- #478: extension.js initializationOptions wiring (depends on this)
- #479: LSP server LspConfig + handler behavior (depends on #478)
- #480: README settings table (depends on #479)

---

## Summary

`extensions/arukellt-all-in-one/package.json` に arukellt の追加設定 5 項目を宣言する。
現在 4 設定 (enableCodeLens, hoverDetailLevel, diagnostics.reportLevel, useSelfHostBackend)
は既に追加済み。5 番目の `arukellt.check.onSave` が欠落している。
この issue では manifest 宣言を完成させる。
**LSP への配線や挙動変更は行わない** (それは #478, #479 の担当)。

## Why this is a separate issue

`package.json` への設定追加は VSCode が設定 UI に表示するための manifest だけを変える。
LSP への配線なしに設定が存在しても、VS Code 設定画面には出るが挙動は変わらない。
この「manifest にある」という状態を単独で done にできるようにするために分離する。
これにより #479 の LSP 実装前に settings UI だけ先行させる罠を防ぐ。

## Visibility

internal-only (VSCode settings UI に表示されるだけ; ユーザー体験に影響しない単独では)

## Primary paths

- `extensions/arukellt-all-in-one/package.json` — `contributes.configuration.properties`

## Allowed adjacent paths

- なし (他ファイルへの変更はこの issue の scope 外)

## Non-goals

- extension.js での設定読み取り (#478)
- LSP サーバーへの設定値の受け渡し (#478)
- LSP サーバー側の挙動変更 (#479)
- README の更新 (#480)
- 設定変更が動作に反映されること (その確認は #479 のclose gate)

## Acceptance

1. `package.json` の `contributes.configuration.properties` に `arukellt.check.onSave` が存在する
   (`type: boolean`, `default: true`, `description` 設定済み, `scope: resource`)
2. 他の 4 設定 (enableCodeLens, hoverDetailLevel, diagnostics.reportLevel, useSelfHostBackend) も
   同じ `properties` ブロックに存在し、型・デフォルト・description・scope が全て設定されている
3. `npm run lint` または同等の package.json バリデーションが pass する
4. `package.json` への変更以外のファイルは変更されていない

## Required verification

- `grep -c "arukellt\." extensions/arukellt-all-in-one/package.json` が少なくとも 5 + 既存設定分を返す
- `grep "check.onSave" extensions/arukellt-all-in-one/package.json` が 1 件ヒットする
- `node -e "JSON.parse(require('fs').readFileSync('extensions/arukellt-all-in-one/package.json','utf8'))"` が exit 0

## Close gate

- `extensions/arukellt-all-in-one/package.json` に 5 設定が全て存在することを grep で確認できる
- `arukellt.check.onSave` が JSON として valid なスキーマで存在している
- **他ファイルへの変更がないこと** (`git diff --name-only` が `package.json` のみを示す)
- LSP 挙動や extension.js の変更はこの issue の close 条件ではない

## Evidence to cite when closing

- `extensions/arukellt-all-in-one/package.json:contributes.configuration.properties` の該当 5 行

## False-done risk if merged incorrectly

- extension.js で設定を読んでいなくても package.json に宣言があれば「done」にできてしまう
  → これは意図通り: この issue は manifest 宣言のみを担当する
- LSP が設定を無視していても close できる → これは意図通り: LSP は #479 で担当
- 設定が動作に影響しないのに「設定が使える」と docs に書かれてしまう
  → docs は #480 が担当; #480 の close gate は #479 完了後

## Closed

- `grep "check.onSave" extensions/arukellt-all-in-one/package.json` → 1 match (`"arukellt.check.onSave": {`)
- `node -e "JSON.parse(require('fs').readFileSync('extensions/arukellt-all-in-one/package.json','utf8'))"` → exit 0 (valid JSON)
- `git diff --name-only HEAD` → only `extensions/arukellt-all-in-one/package.json`
- `bash scripts/run/verify-harness.sh --quick` → 19/19 passed