---
Status: blocked
Created: 2026-03-28
Updated: 2025-07-15
ID: 31
Track: component-model
Depends on: 30
Orchestration class: implementation-ready
Blocks v2 exit: "yes (jco 完了条件)"
Blocked by: "jco upstream (<https://github.com/bytecodealliance/jco>)"
# jco: "Wasm GC 型サポート待ち (upstream blocked)"
### Option A: scalar-only component 向け「GC-free core」モード
### Option B: "jco が GC 対応するまで待機 (現在の選択)"
### Option C: canonical ABI string adapter を実装して jco の GC 依存を迂回
---
# jco: Wasm GC 型サポート待ち (upstream blocked)

## Summary

jco (JavaScript 向け Wasm Component Model ツールチェーン) が Wasm GC 型を含む
コンポーネントのトランスパイルに対応するまで、Arukellt コンポーネントを JavaScript
から呼び出すことができない。jco 側の対応を待つブロック issue。

Arukellt 側での対応タスクは #036 に実装済み。本 issue は**外部依存によるブロック**であり、
Arukellt リポジトリ側では現状できることはない。

---

## 背景

Arukellt の T3 バックエンドは Wasm GC-native であり、スカラー型のみをエクスポートする
関数でも、コア Wasm モジュールの type section に GC 型定義 (string array, vec struct など)
が含まれる。

jco v1.16.1 / v1.17.5 はこれを transpile しようとすると以下のエラーで失敗する:

```
array indexed types not supported without the gc feature
```

このエラーは jco の `wasm-tools` ベースのトランスパイラが GC 型の indexed type
(array, struct) を処理できないことに起因する。

---

## ブロック条件

| 条件 | 内容 |
|------|------|
| ブロック元 | jco が `--gc` フラグまたはデフォルトで Wasm GC 型を含むコンポーネントを transpile できるようになること |
| 追跡方法 | jco releases を定期確認。`npx jco --version` + smoke test でリグレッション検知 |
| 代替手段 | wasmtime CLI (`--wasm gc --wasm component-model`) での検証 (#036 で実装済み) |

---

## jco 側の現状

- jco 1.13.2 (July 2025) で Wasm GC transpile に対応済み（transpiler として）
- Wasmtime 35.0+ LTS で Wasm GC 完全サポート
- Chrome / Firefox / Edge の安定版が Wasm GC 対応済み
- Node.js v20+ で段階的サポート中
- **確認日: 2025-07-15** — unblock 条件 #1 が満たされている可能性が高い
- jco は内部で `wasm-tools` を使用。wasm-tools も GC 対応済み
- ローカルで Arukellt コンポーネント出力に対する jco transpile テストが必要

---

## Arukellt 側の選択肢 (検討用)

以下は Arukellt 側で取り得るアプローチ。いずれも大きなトレードオフがある。

### Option A: scalar-only component 向け「GC-free core」モード

スカラー型のみをエクスポートする関数については、GC 型定義を type section から
省略した T1 互換コア Wasm を生成し、それを component wrap する。

- **利点**: jco でのトランスパイルが即座に可能
- **欠点**: T3 GC-native の設計原則と矛盾。コード分岐が増加。
  String/Vec を含む関数には適用不能。

### Option B: jco が GC 対応するまで待機 (現在の選択)

jco 上流の対応を待つ。wasmtime での検証は #036 で済んでいる。

- **利点**: Arukellt 側に変更不要
- **欠点**: jco での JavaScript 呼び出しがブロックされ続ける

### Option C: canonical ABI string adapter を実装して jco の GC 依存を迂回

\#029 の canonical ABI lift/lower を完全実装し、component ラッパーに
GC → linear memory 変換アダプタを埋め込む。これにより GC 型を内部に隠蔽し、
ABI 境界では linear memory 型のみを expose できる可能性がある。

- **利点**: jco 依存を排除しつつ JavaScript 呼び出しを実現できる可能性
- **欠点**: canonical ABI アダプタの完全実装が必要 (大規模作業)。
  実現可能かどうかは wasm-tools の component model 仕様への適合性に依存。

---

## 解除条件

以下のいずれかが満たされたとき、本 issue を `issues/open/` に移して実装を再開する:

1. `npx jco transpile calculator.component.wasm -o ./out` が Arukellt T3 コンポーネントで成功する
2. Option C (canonical ABI アダプタ) の実現可能性が確認され、実装を決定する

---

## 関連ファイル

- `tests/component-interop/jco/calculator/calculator.ark` — テスト fixture
- `tests/component-interop/jco/calculator/run.sh` — wasmtime 代替テスト
- `docs/process/roadmap-v2.md` §8 完了条件 #3
- `issues/done/036-jco-javascript-interop.md` — 実装済み部分