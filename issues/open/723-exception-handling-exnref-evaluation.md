---
Status: open
Created: 2026-07-15
Updated: 2026-07-15
ID: 723
Track: language-design
Depends on: none
Orchestration class: design-required
Orchestration upstream: none
Blocks v{N}: none
Priority: 4
Source: ADR-008 改訂（2026-07）— Exception Handling (exnref) は Wasm 3.0 で shipped 済み
---

# Wasm Exception Handling (`exnref`) 統合の検討

## Summary

Wasm Exception Handling (`exnref` / `exceptionsFinal`) は Wasm 3.0 で
Phase 5 shipped 済み（ADR-008 改訂）。wasmtime 46、V8 14.6 でデフォルト有効。

現在 Arukellt は `Result<T, E>` 値渡しモデルを採用しており、Wasm 例外命令
（`throw`/`catch`/`exnref`）は一切使用していない。

本 issue は `exnref` を Arukellt に統合するかどうかを**検討**することを目的とする。
実装は別 issue とする。

## Current state

### エラーハンドリングの現状

- `docs/language/error-handling.md:14-19` — 「例外ベースではなく Result<T,E> ベース」
- `src/compiler/mir/lower/try.ark:16-72` — `?` 演算子は tag チェック → early return に lowering
- Result 型は値ベース enum（GC target では GC struct with tag + payload）
- Wasm 例外命令（`OP_THROW`/`OP_CATCH`/`OP_TRY`）は未定義・未使用

### Result モデルの利点

1. 関数シグネチャにエラー型が明示される（型安全性）
2. 正常系のオーバーヘッドが小さい（tag チェックのみ）
3. 決定論的セマンティクス（例外の伝播順序が自明）
4. `?` 演算子で簡潔に書ける

### exnref モデルの利点

1. スタック巻き戻しがネイティブ（深いコールスタックで有利）
2. JS 側が投げた例外を Wasm 側でキャッチできる
3. C FFI 境界での例外漏れを型システムで防げる
4. 例外オブジェクトを GC 管理下の struct として定義可能

## Evaluation points

### 1. 言語設計への影響

`throw`/`catch` を言語機能として追加する場合:

- **関数シグネチャ**: 「例外を投げうるか」の情報を型に含めるか？
  - Rust の `Result` はシグネチャに含まれる（明示的）
  - Java/C++ の checked/unchecked 例外は議論が分かれる
  - Arukellt の設計哲学（明示的・型安全）では Result が合致
- **`?` 演算子との整合**: 現在の `?` は Result の early return に lowering される。
  `throw`/`catch` を入れると2つのエラーモデルが混在する

### 2. 性面での比較

| 側面 | Result<T,E> | exnref |
|------|------------|--------|
| 正常系のオーバーヘッド | tag チェック（小） | ゼロ（throw しなければコストなし） |
| 異常系のオーバーヘッド | tag チェック + early return | スタック巻き戻し（大きい） |
| 深いコールスタック | 全フレームで tag チェック | throw 点から catch 点まで一気に巻き戻し |
| バイナリサイズ | Result 型の struct 定義 | try/catch の制御フロー |

### 3. JS 相互運用のユースケース

`exnref` を使うべき唯一の具体的場面:

```
// 将来の JS interop 案（仮想）
extern fn call_js(fn_name: String) -> Result<JsValue, JsError>
// JS 側が throw した例外を Wasm 側で catch して Result に変換
```

この場合、言語全体のエラーモデルは Result のまま、**FFI 境界の実装詳細として**
`exnref` を使う価値がある。ユーザーに `throw`/`catch` 構文を公開しない。

### 4. Component Model との関係

WASI P2/P3 の component model では、host 側がエラーを返す際に
`result<T, E>` 型（WIT レベル）を使う。これは Arukellt の Result モデルと
直接対応する。`exnref` は component model のエラー伝播とは独立。

## Decision options

| 選択肢 | 内容 | 推奨度 |
|--------|------|--------|
| A: 導入しない | `Result<T,E>` モデルを維持。`exnref` は一切使わない | ✅ **推奨**（現状維持） |
| B: FFI 境界のみ | 言語機能としては `throw`/`catch` を提供せず、JS interop の実装詳細として `exnref` を使用 | 条件付き（JS interop が具体化した時） |
| C: 言語機能として導入 | `throw`/`catch` 構文を追加。Result と例外の2つのモデルが共存 | ❌ 非推奨（設計複雑化） |

## Acceptance criteria

- [ ] Result モデル vs exnref モデルの性能比較（ベンチマック）が実施される
- [ ] JS interop のユースケースが具体化しているか評価される
- [ ] 選択肢 A/B/C のいずれかが推奨として記録される
- [ ] 選択肢 B を採用する場合、JS interop の設計 ADR との整合が確認される
- [ ] ADR-008 #6 の推奨事項が本 issue の結論に合わせて更新される

## Note

- 本 issue は**検討・評価**が目的。実装は別 issue とする
- ADR-008 改訂では「JS 相互運用のユースケースが具体化した時点で設計を開始。
  それまでは `Result<T, E>` を維持」としている
- 优先度は低い（JS interop が具体化するまでは動かないため）

## Related

- ADR-008: WasmGC Post-MVP 拡張機能（#6 Exception Handling 統合）
- ADR-039: Question Mark Operator (`?`) and Error Conversion
- `docs/language/error-handling.md`: エラーハンドリング基本方針
- ADR-007: コンパイルターゲット整理（wasm32-gc）
