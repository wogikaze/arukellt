---
Status: done
Created: 2026-07-11
Updated: 2026-07-11
ID: 761
Track: docs-audit
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 1
Source: Docs audit 2026-07-11 (P0-1)
Blocks: 764, 765
---

# 761 — Docs P0: fixture / verification numeric SSOT

## Summary

`project-state.toml` の fixture 件数（1199）と `manifest.txt` 実数（2679）、
生成 README（2679）、`test-strategy.md`（434）が三重管理になっている。
freshness 検査は存在しないキー `fixture_count` を見ておりドリフトを検知できない。
root README の snapshot は fixture failures を省略する。

## Acceptance

- [x] `docs/data/project-state.toml` の `fixture_manifest_count` が `tests/fixtures/manifest.txt` の非コメント行数と一致
- [x] `generate-docs.py` の section snapshot / README / current-state は `project-state.toml` の verification フィールドのみから数値を出す（`fixture_count()` 再集計を表示に使わない）
- [x] README / docs README の harness 行は `passed / failed / skipped / total` を同一形式で出す
- [x] `check-docs-freshness.py` / `check-docs-consistency.py` が `fixture_manifest_count` を検査する
- [x] `docs/test-strategy.md` の手書き 434 を正本参照に置換
- [x] Docs-related verify gates for this slice pass; remaining verify failures are pre-existing infra (host-linker / bootstrap wasm / gate-648)

## References

- Docs audit 2026-07-11 §P0-1
- `docs/data/project-state.toml`
- `scripts/gen/generate-docs.py`
- `scripts/check/check-docs-freshness.py`


## Completion

Completed 2026-07-11 as part of docs audit remediation (Stage 1 + quick Stage 2).
