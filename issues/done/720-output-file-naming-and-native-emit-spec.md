---
Status: closed
Created: 2026-07-07
Updated: 2026-07-07
ID: 720
Track: spec
Depends on: none
Orchestration class: spec
Orchestration upstream: none
Blocks v{N}: none
Priority: 2
Source: ADR-007 改定（2026-07）— 出力ファイル命名規則と native 出力形式が未決定
---

# 出力ファイル命名規則と native emit 形式の決定

## Summary

ADR-007 改定（2026-07）にて、ターゲット構造を `wasm32` / `wasm32-gc` /
`native-cpp` / `native-llvm` の3系統に再構成した。この改定で
出力ファイル名は `<input>.*` の仮置きとし、native の出力形式は
「別途ADRで決定」とした。本 issue は以下の2点を決定する:

1. **出力ファイル命名規則**: `<input>.*` の最終形
2. **native-cpp / native-llvm の出力形式**: `.s`, `.ll`, `.o`, `.out` 等

## Context

### 出力ファイル命名規則

現在の仮置き:

| ターゲット | 出力 |
|-----------|------|
| `wasm32` | `<input>.wasm`, `<input>.wat` |
| `wasm32-gc` | `<input>.wasm`, `<input>.wat`, `<input>.wit`, `<input>.component.wasm`, `<input>.core.wasm`, `<input>.world.wit` |
| `native-cpp` / `native-llvm` | 別途ADRで決定 |

決定すべき点:
- `<input>` の具体的内容（入力ファイル名の stem？ `--output` で上書き可？）
- `--emit all` 時の複数ファイル命名（`<input>.wasm` + `<input>.component.wasm` + `<input>.wit` が衝突しないか）
- `world.wit` の命名（`<input>.world.wit` か `world.wit` か `<input>.wit` か）
- Component化・jco transpile 後の中間ファイルの命名

### native 出力形式

決定すべき点:
- `native-cpp`: `.cpp` / `.cc` / `.c` / `.s` (アセンブラ) のどれを出力するか
- `native-llvm`: `.ll` (LLVM IR テキスト) / `.bc` (LLVM bitcode) / `.s` (アセンブラ) のどれを出力するか
- リンクまで実行するか（`.out` / 実行ファイルを生成するか）、コンパイルのみか
- これは ADR-005（LLVM 従属）の範囲内で決定する

## Acceptance criteria

- [x] 出力ファイル命名規則が ADR-007 に追記される
      — 2026-07-07: ADR-007 §出力ファイル を全面改訂。`<input>` 定義、
      ターゲット別出力ファイル表、`--output` 上書き規則を明文化。
- [x] native-cpp / native-llvm の出力形式が ADR-007 または新規ADRに記載される
      — 2026-07-07: ADR-007 §native ターゲットの出力形式 に追記。
      native-cpp → `.c` (C99), native-llvm → `.ll` (LLVM IR)。
      object/executable/bitcode emit kind も定義。
      Emit surface 表も native 列を追加。
- [x] `<input>` の定義（stem抽出、`--output` 上書き可否）が明文化される
      — 2026-07-07: `<input>` = 入力ファイルパスから `.ark` 拡張子を除去した文字列。
      `--output <path>` で上書き可能（`--emit core-wasm` 時はそのまま、
      `--emit all` 時は `.wasm`/`.component.wasm` を派生）。
- [x] `--emit all` 時のファイル衝突がないことを確認
      — 2026-07-07: `--emit all` は `wasm32-gc` のみ有効で
      `<input>.wasm` + `<input>.component.wasm` を生成。拡張子が異なるため衝突なし。
      `<input>.wit` は `--emit wit` で別途出力。

## Related

- ADR-007: コンパイルターゲット整理（出力ファイルセクション）
- ADR-005: LLVM バックエンドの役割制限（native-llvm の範囲）
- ADR-006: 公開 ABI 3層構造
