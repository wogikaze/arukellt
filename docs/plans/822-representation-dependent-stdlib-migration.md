# #822 — Representation-dependent and allocating stdlib migration クローズ計画

ステータス: **done**（Vec/String/parse/format/sort 完了。SIMD×3 は #698 へ carve-out）。
legacy_emitter: **31 → … → 10 → 3**（残は `simd.i32x4.add/sub`, `simd.f32x4.add` のみ）。

親 issue: [#822](../../issues/done/822-representation-dependent-stdlib-migration.md)  
前提: #798, #816, #817, #820 done  
担当 subagent lane: `wave/822-repr-stdlib`  
作業 worktree: `.worktrees/wave-822-repr-stdlib`  
作成日: 2026-07-25  
完了日: 2026-07-26

## 完了サマリ

- sealed raw: `raw.array_new` / typed LM `raw.array_grow`（stride + shrink len）/
  `raw.array_set_len`（CoreOp 追加；pop は当面 grow 縮退を使用）。
- Vec: `push`/`push_i64`/`push_f64`/`get`/`pop`/`Vec_new_*` を `normal_call` 化。
- 検証: `test_stdlib_inline` OK、`verify lane --gate t3` OK。
- SIMD portable 3 ops は ADR-037 / #698 へ正式移管（#822 Non-goals）。

## 検証コマンド

```bash
python3 scripts/tests/test_stdlib_inline.py
python3 scripts/manager.py verify lane --gate t3
```
