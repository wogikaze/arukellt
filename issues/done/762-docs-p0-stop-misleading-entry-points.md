---
Status: done
Created: 2026-07-11
Updated: 2026-07-11
ID: 762
Track: docs-audit
Depends on: 761
Orchestration class: implementation-ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 1
Source: Docs audit 2026-07-11 (P0-2, P0-3)
Blocks: 764, 765
---

# 762 — Docs P0: stop misleading current entry points

## Summary

`overview.html`、`process/policy.md`、`process/decision-guide.md`、
`release-criteria.md` が current 区画で旧ターゲット体系（T1–T5 /
`wasm32-wasi-p1` primary 等）を断言している。入口から誤誘導を止める。

## Acceptance

- [x] `overview.html` に archive / stale banner。docs README / sidebar の「初見向け」扱いをやめる
- [x] `process/policy.md` の target / emit 表を ADR-007/013 canonical 名に更新
- [x] `process/decision-guide.md` の production path を `wasm32-gc` に修正
- [x] `release-criteria.md` の `wasm32` Experimental 表記を current-state の supported/stable と整合
- [x] current 区画の意思決定文書で旧 T0–T5 を現行表として使わない（alias 表は隔離可）
- [x] Docs-related verify gates for this slice pass; remaining verify failures are pre-existing infra (host-linker / bootstrap wasm / gate-648)

## References

- Docs audit 2026-07-11 §P0-2, P0-3
- `docs/current-state.md`
- `docs/adr/ADR-007-targets.md`
- `docs/adr/ADR-013-primary-target.md`

## Completion

Completed 2026-07-11 as part of docs audit remediation (Stage 1 + quick Stage 2).
