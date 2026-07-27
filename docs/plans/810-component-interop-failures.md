# #810 — Component interop failures クローズ計画

ステータス: 計画  
親 issue: [#810](../../issues/open/810-component-interop-failures.md)  
担当 subagent lane: `wave/810-component-interop`  
作業 worktree: `.worktrees/wave-810-component-interop`  
作成日: 2026-07-25

## 1. 現状とゴール

- `python3 scripts/manager.py verify full` で 103 件の component interop fixture が失敗。
- 目標: すべての component interop fixture が `wasm-tools validate --component` と wasmtime 呼び出しを通過。

## 2. 前提・依存

- なし。
- 主なコードは `src/compiler/component/` 以下。

## 3. フェーズと完了条件

### Phase 0 — 失敗リスト取得と分類
- `docs/data/verify-full-receipt.json` から `component_interop` の失敗 item を抽出。
- 型カテゴリ別に分類:
  - primitives
  - option / result
  - list / string
  - record / tuple / variant

### Phase 1 — バリデーション優先
- プリミティブ型の export を修正。
- `naming.ark` の snake_case → kebab-case 変換を確認。
- `export_types.ark` / `types_wit_func.ark` の WIT 型エンコーディングを修正。

### Phase 2 — 複合型アダプター実装
- Option / Result / List / String / Record / Tuple の canonical ABI lift/lower アダプターを生成。
- 1 カテゴリごとに `python3 scripts/manager.py verify component-interop` を実行し、失敗数を減らす。

### Phase 3 — 最終検証
- 全 103 fixture 通過。
- `python3 scripts/manager.py verify full` で回帰確認。

## 4. 作業レーン・並列可否

- `src/compiler/component/` 以下に集中し、他レーンとはファイル競合が少ない。
- ただし自己ホストビルドは `runtime_lock` で直列化される。親オーケストレータが `build-compiler` を集中管理。

## 5. 検証コマンド

```bash
python3 scripts/manager.py verify component-interop
wasm-tools validate --component <component.wasm>
bash tests/component-interop/jco/bool-logic/run.sh
python3 scripts/manager.py verify full
```

## 6. リスク

- スコープが大きく、数週間〜数ヶ月かかる可能性。
- WIT Canonical ABI のメモリ管理・lift/lower が複雑。
- アダプターモジュール生成が不完全。

## 7. 進捗更新規則

- カテゴリごとに失敗数を記録。
- 各カテゴリ通過後に `verify component-interop` の結果を issue に追記。