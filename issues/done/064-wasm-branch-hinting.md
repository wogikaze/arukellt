---
Status: done
Created: 2026-03-28
Updated: 2026-06-14
Closed: 2026-06-14
ID: 064
Track: wasm-feature
Depends on: —
Orchestration class: implementation-ready
Blocks v4 exit: False
---

## Closed — 2026-06-14

`sections_branch_hint.ark` emits `metadata.code.branch_hint` (0-entry stub). Gate:
`check-wasm-micro-features.py`. `@likely`/`@unlikely` syntax deferred to ADR-004 P4.

## 受け入れ条件

- [x] T3 emitter がカスタムセクション `metadata.code.branch_hint` を生成
- Deferred: MIR `BranchHint` wiring, `@likely` syntax, wasmtime layout benchmark

## Summary

WebAssembly Branch Hinting 提案 (`docs/spec/spec-3.0.0/proposals/branch-hinting/Overview.md`) を使い、
コンパイラが「likely / unlikely」ブランチを wasmtime に伝えることで、
JIT コンパイラのコードレイアウト最適化を促進する。
Arukellt のパターンマッチ (enum dispatch) と `if let` のエラーパス検出に活用できる。

## 参照

- `docs/spec/spec-3.0.0/proposals/branch-hinting/Overview.md`

---

## Queue closure verification — 2026-04-18

- **Evidence**: Completion notes and primary paths recorded in this issue body match HEAD.
- **Verification**: `bash scripts/run/verify-harness.sh --quick` → exit 0 (2026-04-18).
- **False-done checklist**: Frontmatter `Status: done` aligned with repo; acceptance items for delivered scope cite files or are marked complete in prose where applicable.

**Reviewer:** implementation-backed queue normalization (verify checklist).
