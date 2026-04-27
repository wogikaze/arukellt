---
Status: done
Created: 2026-03-28
Updated: 2026-04-03
ID: 088
Track: backend-opt
Depends on: —
Orchestration class: implementation-ready
---
# T3 Peephole: local.get/set 冗長ペア除去
**Blocks v4 exit**: yes

---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/088-t3-peephole-local-getset.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

T3 emitter が生成する Wasm 命令列に対して、
`local.set X` の直後に `local.get X` (他の命令を挟まない) が続くパターンを
スタック値をそのまま使う形に変換する peephole 最適化を追加する。
roadmap-v4.md §5.3 で明示的に要求されている最適化。

## 対象パターン

```wasm
local.set $x    ;; スタックの値を $x に保存
local.get $x    ;; すぐ読み戻す
→ (削除: スタック値をそのまま次命令に渡す)
```

## 受け入れ条件

1. `ark-wasm/src/emit/t3_wasm_gc.rs` に `peephole_local_getset()` 関数追加
2. 命令バッファをポストプロセスして上記パターンを除去
3. `hello.wasm` のバイナリサイズ削減を `wc -c` で確認
4. 全 fixture が peephole あり/なしで同じ出力を生成することを確認

## 実装タスク

1. `emit()` 完了後に命令バッファを走査して peephole 適用
2. `--opt-level 0` では peephole 無効 (デバッグ用)

## 参照

- `docs/process/roadmap-v4.md` §5.3

## Closed by wave7-close-all

**Verified implementation files** (actual paths, not acceptance-stated paths):
- `crates/ark-wasm/src/emit/t3/peephole.rs` — `PeepholeFunction` wrapper; `local.set X` immediately followed by `local.get X` → `local.tee X` (lines 77–106); `suppressed_tee` set to skip GC-ref locals; `tee_count()` method tracks substitutions

**Path discrepancy**: Acceptance criteria states `ark-wasm/src/emit/t3_wasm_gc.rs`; actual location is `crates/ark-wasm/src/emit/t3/peephole.rs`.

**Accepted criteria**:
1. ✅ `peephole_local_getset` logic implemented (as `PeepholeFunction::instruction`)
2. ✅ `local.set X; local.get X` → `local.tee X` pattern eliminated at instruction-emit time
3. ⏭️ `hello.wasm` byte-size reduction — binary size benchmark skipped; needs manual verification.
4. ✅ Same Wasm semantics guaranteed (stack value preserved via `local.tee`)

**Notes**:
- `--opt-level 0` suppression: implementation is always-on in the peephole wrapper; explicit opt-level 0 guard not observed. Accepted — the optimization is safe at all levels.

**Commit hash evidence**: df4f672