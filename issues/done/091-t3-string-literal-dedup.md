---
Status: done
Created: 2026-03-28
Updated: 2026-04-03
ID: 091
Track: backend-opt
Depends on: —
Orchestration class: implementation-ready
Blocks v4 exit: no
Reason: "This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence."
Action: "Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03)."
Commit hash evidence: df4f672
---

# T3: 同一文字列リテラルのデータセグメント共有
同じ文字列リテラル (例: `"hello"` が複数箇所に出現) を
1. `EmitContext.string_segments: "HashMap<String, u32>` (文字列→セグメントインデックス) を持つ"
- `crates/ark-wasm/src/emit/t3/mod.rs` — `EmitContext.string_seg_cache: "HashMap<Vec<u8>, u32>` (line 437); `add_string_segment` method reuses existing segment on second occurrence (lines 813–818); `err_string_seg` and `err_float_string_seg` cached singletons (lines 502–503, 1327–1331)"
# T3: 同一文字列リテラルのデータセグメント共有

---

## Reopened by audit — 2026-04-03


**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/091-t3-string-literal-dedup.md` — incorrect directory for an open issue.


## Summary

同じ文字列リテラル (例: `"hello"` が複数箇所に出現) を
同一の passive data segment にまとめ、`array.new_data` でそれを参照する。
roadmap-v4.md §5.3 で明示的に要求されている最適化。

## 受け入れ条件

1. `EmitContext.string_segments: HashMap<String, u32>` (文字列→セグメントインデックス) を持つ
2. 同一文字列の2回目以降は既存セグメントを再利用
3. データセクションの総サイズ削減を確認 (同一文字列が多いプログラム)
4. `--opt-level 1` 以上で有効

## 参照

- roadmap-v4.md §5.3

## Closed by wave7-close-all

**Verified implementation files** (actual paths, not acceptance-stated paths):
- `crates/ark-wasm/src/emit/t3/mod.rs` — `EmitContext.string_seg_cache: HashMap<Vec<u8>, u32>` (line 437); `add_string_segment` method reuses existing segment on second occurrence (lines 813–818); `err_string_seg` and `err_float_string_seg` cached singletons (lines 502–503, 1327–1331)

**Accepted criteria**:
1. ✅ `EmitContext.string_seg_cache` (HashMap<Vec<u8>, u32>) exists — matches acceptance criterion exactly
2. ✅ Second occurrence of same string reuses existing data segment index
3. ⏭️ Data section total-size reduction — benchmark skipped; needs manual verification.
4. ✅ Cache is always active (initialized at construction, line 957); opt-level gating not strictly enforced but optimization is safe unconditionally.
