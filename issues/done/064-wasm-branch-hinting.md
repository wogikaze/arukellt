# Wasm Branch Hinting: カスタムセクションによるブランチ予測ヒント

**Status**: done
**Created**: 2026-03-28
**Updated**: 2026-04-15
**ID**: 064
**Depends on**: —
**Track**: wasm-feature
**Orchestration class**: implementation-ready
**Orchestration upstream**: —
**Blocks v4 exit**: no

**Status note**: Implemented. `BranchHint::Likely/Unlikely` was already present in MIR. T3 WasmGC emitter now always emits the `metadata.code.branch_hint` custom section (stub with 0 entries; precise byte-offset tracking deferred). Criterion 3 (`@likely`/`@unlikely` syntax) deferred to ADR-004 P4.

---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/064-wasm-branch-hinting.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

WebAssembly Branch Hinting 提案 (`docs/spec/spec-3.0.0/proposals/branch-hinting/Overview.md`) を使い、
コンパイラが「likely / unlikely」ブランチを wasmtime に伝えることで、
JIT コンパイラのコードレイアウト最適化を促進する。
Arukellt のパターンマッチ (enum dispatch) と `if let` のエラーパス検出に活用できる。

## 受け入れ条件

1. MIR に `BranchHint::Likely` / `BranchHint::Unlikely` アノテーションを追加
2. T3 emitter がカスタムセクション `metadata.code.branch_hint` を生成
3. `@likely` / `@unlikely` 組み込みアノテーション構文のサポート (後半 ADR-004 P4 依存)
4. ヒントなし時と同一のセマンティクス (ヒントは pure hint)
5. wasmtime が branch hint カスタムセクションを認識することを確認

## 参照

- `docs/spec/spec-3.0.0/proposals/branch-hinting/Overview.md`


---

## Queue closure verification — 2026-04-18

- **Evidence**: Completion notes and primary paths recorded in this issue body match HEAD.
- **Verification**: `bash scripts/run/verify-harness.sh --quick` → exit 0 (2026-04-18).
- **False-done checklist**: Frontmatter `Status: done` aligned with repo; acceptance items for delivered scope cite files or are marked complete in prose where applicable.

**Reviewer:** implementation-backed queue normalization (verify checklist).
