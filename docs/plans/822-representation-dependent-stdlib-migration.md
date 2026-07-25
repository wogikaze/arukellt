# #822 — Representation-dependent and allocating stdlib migration クローズ計画

ステータス: 進行中（+ format_f64 / push_char; legacy 31→28→25→**23**）。f32_to_string / parse / Vec / sort / SIMD は阻害で残。#822 open 維持。  

親 issue: [#822](../../issues/open/822-representation-dependent-stdlib-migration.md)  
前提: #798, #816, #817, #820 done（#820 は WAT export 正規表現修正後）  
担当 subagent lane: `wave/822-repr-stdlib`  
作業 worktree: `.worktrees/wave-822-repr-stdlib`  
作成日: 2026-07-25

## 1. 現状とゴール

- Vec/String などの表現依存・allocating 操作を emitter から Ark stdlib 本体に移行する。
- 残る `legacy_emitter` CoreOp を `normal_call`  lowering に変更し、sealed raw API 越しに実装する。
- 目標: `data/core-ops.toml` に `legacy_emitter` ゼロ、`verify quick` 0 失敗。

## 2. 前提・依存

- #817 sealed raw API 完了。
- #820 inliner test regex 修正後、`test_stdlib_inline` を通してから新規 probe を追加。
- ジェネリック Vec 変異は fallback resolver 拡張が必要なため、今回の wave では concrete 操作を優先。

## 3. フェーズと完了条件

### Phase 1 — 残り CoreOp インベントリ
- `data/core-ops.toml` から `lowering.kind = "legacy_emitter"` の操作を列挙。
- 以下のうち、今回対象とするものを選定:
  - 数値パース: `parse.parse_i32`, `parse.parse_i64`, `parse.parse_f64`
  - 浮動小数点フォーマット: `text.f32_to_string`, `text.format_bool`, `text.char_to_string`
  - スカラー: `scalar.f64_bits_hi`, `scalar.f64_bits_lo`
  - シーケンスソート: `seq.sort_i64`, `seq.sort_f64`
  - その他 concrete 操作: `core.range_new`, `math.sqrt`
- ジェネリック Vec 変異 / SIMD portable は別 issue または次 wave に委譲。

### Phase 2 — sealed raw API 越しの実装追加
- `std/collections/string.ark` / `std/collections/vec.ark` に private `__core_*_impl` 関数を追加。
- `data/core-ops.toml` の `lowering.kind` を `normal_call` に変更。
- `[operations.fallback]` に `implementation_symbol` と `required = true` を追加。

### Phase 3 — inliner differential probe 追加
- #820 修正後、`scripts/tests/test_stdlib_inline.py` に移行済み操作の probe を追加。

### Phase 4 — 検証
- `python3 scripts/tests/test_stdlib_inline.py`
- `python3 scripts/manager.py verify quick`

## 4. 作業レーン・並列可否

- `#821` done なので独立可能。
- `#819` / `#818` とは `data/core-ops.toml` の競合リスクがある。マージは親オーケストレータが調整。

## 5. 検証コマンド

```bash
python3 scripts/tests/test_stdlib_inline.py
python3 scripts/manager.py verify lane --gate t3
python3 scripts/manager.py verify quick
```

## 6. リスク

- CoreOp metadata（effect, trap, ordering）の保存漏れ。
- ジェネリック Vec 操作を無理に移行すると GC reference/value 型不一致が発生。
- `verify quick` の回帰。

## 7. 進捗更新規則

- 1 つの CoreOp ファミリー移行ごとに commit。
- 最終的に `data/core-ops.toml` の legacy カウントを issue 本文に記録する。