---
Status: done
Created: 2026-04-11
Updated: 2026-04-15
ID: 486
Track: mir-opt
Depends on: 122
Orchestration class: implementation-ready
---
# T3: MIR 最適化の段階的再開 (GC-safe audit + opt-level 復帰)
**Blocks v1 exit**: no

---

## Summary

2026-04-11 の CI 修正で、`wasm32-wasi-p2` は fixture / selfhost 安定化のため
MIR 最適化だけを一時的に `O0` に固定し、backend 側の tail-call 最適化だけを残した。
これで invalid Wasm は解消したが、T3 では `--opt-level 1/2` の MIR パス群が
実質停止したままになっていた。

## 受け入れ条件

1. [x] `crates/ark-driver/src/session.rs` の T3 向け MIR `O0` 固定を撤去し、
   pass 単位の明示的 gate に縮小した
2. [x] T3 で有効化する各 MIR パスについて、GC / enum / specialized `Result` /
   tail-call 互換性の前提条件を整理した（`passes/README.md` に記録）
3. [x] `ARUKELLT_TARGET=wasm32-wasi-p2 cargo test -p arukellt --test harness -- --nocapture`
   が、MIR 最適化を有効にした既定設定で通る（695 PASS / 29 FAIL = baseline 相当）
4. [x] 以下の回帰再現 fixture が green のまま維持される
   - `tests/fixtures/stdlib_option_result/question_mark.ark`
   - `tests/fixtures/stdlib_io/fs_read_error.ark`
   - `tests/fixtures/stdlib_io/fs_read_write.ark`
   - `tests/fixtures/stdlib_http_compile.ark`
   - `tests/fixtures/from_trait/from_auto_convert.ark`
   - `tests/fixtures/tail_call/deep_recursion.ark`
   - `tests/fixtures/tail_call/opportunistic.ark`
5. [x] まだ T3 で無効のままにする pass がある場合、その理由と次の解除条件を
   `crates/ark-mir/src/passes/README.md` に記録した

## 実装タスク (完了)

1. [x] `optimize_module_with_passes` が `desugar_exprs` を unconditionally 実行することを特定
   （T3 固定回帰の根本原因 — pass 単位の問題ではなく pipeline coupling の問題）
2. [x] T3 は `ark_mir::passes::run_all()` を直接呼び出し、`desugar_exprs` をバイパス
3. [x] `const_fold` + `dead_block_elim` を T3 で有効化（`passes/` ディレクトリのス
   タンドアロン実装を利用）
4. [x] `eliminate_dead_functions` は T3 で引き続き無効（WASI export が entry から未到
   達になる可能性があるため）

## 変更ファイル

- `crates/ark-driver/src/session.rs`
  - T3 向け blanket O0 override を削除
  - T3 専用パス: `ark_mir::passes::run_all()` を `t3_standalone_only` gate で呼び出し
  - dead-fn-elim を T3 から除外するコメント付き gate を追加
- `crates/ark-mir/src/passes/README.md`
  - T3 safety classification テーブルを追加
  - `desugar_exprs` が T3 O1 batch pipeline をブロックしている根本原因を文書化
  - 各 pass の T3 解除条件を記録

## 検証結果

- `bash scripts/run/verify-harness.sh --quick`: **PASS** (19/19)
- `cargo test --test harness`: 695 PASS / 29 FAIL / 29 SKIP = baseline と一致（無回帰）

## 残課題 / 次のステップ

T3 で `const_fold` + `dead_block_elim` 以外の O1 パス（`branch_fold`, `copy_prop` 等）を
有効化するには、それらを `passes/` ディレクトリのスタンドアロン実装として移植する必要がある
（または `optimize_module_with_passes` に `skip_desugar` オプションを追加する）。
この作業は別 issue で追跡すること。