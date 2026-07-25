# #826 — Symbol / path interning + hot-path clone audit クローズ計画

ステータス: Phase 1–2 完了 + NameIndex bounded win（investigation 継続可）  
親 issue: [#826](../../issues/open/826-symbol-path-intern-clone-audit.md)  
前提: #823, #829 done  
担当 subagent lane: `wave/826-intern-clone`  
作業 worktree: `.worktrees/wave-826-intern-clone`  
作成日: 2026-07-25

## 1. 現状とゴール

- stage-3 compile で RSS が線形増加（1.37→2.40 GiB）。
- callee names、type names、module names、canonical IDs での `clone()` が bump-heap 圧力の一因と推定。
- 目標: 計測された hot clone サイトを intern table で削減し、RSS 増加を鈍化または wall time を改善する。

## 2. 前提・依存

- #823 / #829 done。
- `KEEP_CLOCK` / `--time` 機能。

## 3. フェーズと完了条件

### Phase 1 — 計測されたインベントリ作成
- `KEEP_CLOCK` receipt を拡張し、phase ごとの `clone_calls` / `clone_bytes` を計測。
- 対象ファイル:
  - `src/compiler/mir/post_pass_callee_cache.ark`
  - `src/compiler/mir/post_pass_type_propagate.ark`
  - `src/compiler/mir/post_pass_callee_lookup.ark`
  - `src/compiler/mir/module_host_calls.ark`
  - `src/compiler/wasm/sections_imports.ark`
  - `src/compiler/wasm/code_ref_locals_typename.ark`
  - `src/compiler/corehir/core_op_registry.ark`

### Phase 2 — 所有権・ライフタイム設計
- Session-durable intern table を推奨。
- 配置は durable bump heap（phase arena reset 対象外）。
- Interned string は `i32` index で表現。
- `String` フィールドを段階的に `InternedString` に移行。

### Phase 3 — 段階的実装
- 1 phase / 1 サブシステムずつ移行。
- `String` と `InternedString` の相互変換ヘルパーを用意。

### Phase 4 — 検証
- before/after の `clone_calls` / wall time / RSS を比較。
- 成功基準:
  - `clone_calls` 50% 以上削減、または
  - wall time 5% 以上改善、または
  - RSS 増加鈍化
- `python3 scripts/manager.py verify quick` PASS。

## 4. 作業レーン・並列可否

- #807 と並列可能。
- #824 とは `MirModule` / `MirFunction` 構造体変更で競合する可能性がある。
- 単独レーンが最も安全。

## 5. 検証コマンド

```bash
ARUKELLT_OVERLAY_KEEP_CLOCK=1 python3 scripts/manager.py selfhost build-compiler
arukellt compile src/compiler/main.ark --target wasm32-gc --wasi-version wasi-p2 --time > /tmp/before-receipt.json
# 実装後
arukellt compile src/compiler/main.ark --target wasm32-gc --wasi-version wasi-p2 --time > /tmp/after-receipt.json
python3 scripts/manager.py verify quick
python3 scripts/manager.py selfhost build-compiler
```

## 6. リスク

- Bump-heap ライフタイムと #827 phase arena 導入時の再設計。
- `String` と `InternedString` 混在期間のバグ。
- `clone()` コール自体のオーバーヘッドが小さく、効果が計測できない。

## 7. 進捗更新規則

- Phase 1 の計測結果を issue 本文または `docs/research/selfhost-compile-latency-root-cause.md` に追記。
- 効果がない場合は defer として理由を記録。