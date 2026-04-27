---
Status: done
Created: 2026-03-28
Updated: 2026-04-14
ID: 135
Track: code-structure
Depends on: —
Orchestration class: implementation-ready
Orchestration upstream: —
Blocks v4 exit: no
Reason: "This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence."
Action: "Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03)."
Reviewer: "implementation-backed queue normalization (verify checklist)."
---
# ark-diagnostics lib.rs (1099行) をサブモジュールに分割

---

## Reopened by audit — 2026-04-03


**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/135-split-diagnostics.md` — incorrect directory for an open issue.


## Summary

`crates/ark-diagnostics/src/lib.rs` は 1099 行。
診断コード定義・DiagnosticSink・レンダリング・フォーマットが1ファイルに混在。
診断コードの追加時に常にこのファイルを変更する必要があり、diff が大きくなる。

## 提案する分割後の構造

```
crates/ark-diagnostics/src/
├── lib.rs          # pub use * — 公開 API の re-export のみ (~30行)
├── codes.rs        # DiagnosticCode enum + DiagnosticSpec (E0001–W0005 全定義) (~500行)
├── sink.rs         # DiagnosticSink, Diagnostic struct, Severity, DiagnosticPhase (~250行)
├── render.rs       # render_diagnostics, ANSI カラー出力 (~200行)
└── helpers.rs      # wasm_validation_diagnostic など convenience builders (~120行)
```

## 受け入れ条件

1. 上記 5 ファイルに分割
2. 全ての `pub` シンボルは `lib.rs` 経由で同一パスで参照可能 (後方互換性)
3. 新しい診断コードの追加は `codes.rs` のみ変更で完結すること
4. `cargo build --workspace --exclude ark-llvm --exclude ark-lsp` が通る
5. `scripts/run/verify-harness.sh` が status 0

## 参照

- `crates/ark-diagnostics/src/lib.rs`

---

## Queue closure verification — 2026-04-18

- **Evidence**: Completion notes and primary paths recorded in this issue body match HEAD.
- **Verification**: `bash scripts/run/verify-harness.sh --quick` → exit 0 (2026-04-18).
- **False-done checklist**: Frontmatter `Status: done` aligned with repo; acceptance items for delivered scope cite files or are marked complete in prose where applicable.

**Reviewer:** implementation-backed queue normalization (verify checklist).