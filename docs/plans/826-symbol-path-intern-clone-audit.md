# #826 — Symbol / path interning + hot-path clone audit クローズ計画

ステータス: **closed**（2026-07-26）  
親 issue: [#826](../../issues/done/826-symbol-path-intern-clone-audit.md)  
前提: #823, #829 done  
担当 subagent lane: `wave/826-intern-clone`  
作業 worktree: `.worktrees/wave-826-intern-clone`  
作成日: 2026-07-25  
完了日: 2026-07-26

## 1. 現状とゴール

- stage-3 compile で RSS が線形増加（1.37→2.40 GiB）。
- callee names、type names、module names、canonical IDs での `clone()` が bump-heap 圧力の一因と推定。
- 目標: 計測された hot clone サイトを intern table で削減し、RSS 増加を鈍化または wall time を改善する。

## 2. 前提・依存

- #823 / #829 done。
- `KEEP_CLOCK` / `--time` 機能。

## 3. フェーズと完了条件

### Phase 1 — 計測されたインベントリ作成 — **done**

- 静的 inventory + call-path を
  [`docs/research/826-symbol-path-intern-clone-audit.md`](../research/826-symbol-path-intern-clone-audit.md) に記録。
- KEEP_CLOCK `clone_calls` / `clone_bytes` カウンタは **defer**（wall/propagate A/B で代替）。

### Phase 2 — 所有権・ライフタイム設計 — **done**

- Session-durable intern table を提案・文書化（phase arena reset 対象外、`i32` handle）。

### Phase 3 — 段階的実装 — **partial（acceptance 内）**

- NameIndex `find_slot` probe deep-clone 除去を landed（`src/compiler/collections/name_index.ark`）。
- 全面 `InternedString` 移行は acceptance 外 follow-up。

### Phase 4 — 検証 — **done（wall 基準）**

- fair A/B: wall **−21.5%** / propagate **−30.7%**（成功基準 wall −5% 達成）。
- `verify lane` PASS（close 時再実行）。

## 4. 作業レーン・並列可否

- #807 と並列可能。
- #824 とは `MirModule` / `MirFunction` 構造体変更で競合する可能性がある。
- 単独レーンが最も安全。

## 5. 検証コマンド

```bash
ARUKELLT_OVERLAY_KEEP_CLOCK=1 python3 scripts/manager.py selfhost build-compiler
# fair A/B: rebuild with old vs new name_index_find_slot, then
# wasmtime … arukellt-s2-runtime.wasm -- compile src/compiler/main.ark …
python3 scripts/manager.py verify lane
python3 scripts/manager.py docs regenerate
python3 scripts/manager.py docs check
```

## 6. リスク

- Bump-heap ライフタイムと #827 phase arena 導入時の再設計。
- `String` と `InternedString` 混在期間のバグ。
- `clone()` コール自体のオーバーヘッドが小さく、効果が計測できない。

## 7. 進捗更新規則

- Phase 1 の計測結果を issue 本文または research に追記。
- 効果がない場合は defer として理由を記録。

## 8. Close

- Issue → `issues/done/826-symbol-path-intern-clone-audit.md`
- 正本 research: `docs/research/826-symbol-path-intern-clone-audit.md`
- Landing: `d54cf4ad`
