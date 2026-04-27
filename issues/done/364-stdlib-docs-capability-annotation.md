---
Status: done
Created: 2026-03-31
Updated: 2026-04-01
ID: 364
Track: stdlib-docs
Depends on: —
Orchestration class: implementation-ready
---
# Stdlib Docs: host module の target/capability 注記を統一する

## Acceptance

- [x] host family の全 API に target 互換性注記が docs に存在する
- [x] `host_stub` API に明確な「未実装」警告バナーが表示される
- [x] capability 注記フォーマットが統一される (target, required capability, status)
- [x] `scripts/gen/generate-docs.py` が manifest の target/kind から注記を自動生成する

## Resolution

- Updated `scripts/gen/generate-docs.py` to show target constraints and ⚠️ warning for host_stub APIs
- Reference table now shows `host_stub ⚠️ (wasm32-wasi-p2)` for stub functions
- Target-limited functions show their target constraint in the Kind column
- All 3 host_stub functions (get, request, connect) now clearly marked
- Regenerated `docs/stdlib/reference.md` with annotations