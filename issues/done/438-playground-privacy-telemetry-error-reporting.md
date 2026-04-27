---
Status: done
Created: 2026-03-31
Updated: 2026-04-15
ID: 438
Track: playground
Depends on: 437
Orchestration class: implementation-ready
Blocks v1 exit: False
Priority: 11
Reason: No telemetry switch code.
Action: Code-level guardrail added. All acceptance criteria met.
# Playground: privacy / telemetry / error reporting を実装方針付きで定める
---
# Playground: privacy / telemetry / error reporting を実装方針付きで定める

## Reopened by audit — 2026-04-13



## Closed — 2026-04-15


**Evidence**:
- `playground/src/telemetry.ts` — adds `TELEMETRY_DISABLED = true` (compile-time constant),
  `reportError()`, `reportWasmLoadError()`, and `reportCompilerPanic()` as the single
  hook point for all error reporting. v1 logs to console only; no outbound requests.
- Exported from `playground/src/index.ts` as part of the public package API.
- `docs/playground/privacy-telemetry-policy.md` — comprehensive policy (422 lines):
  no data collection, no cookies, no telemetry, fragment-based share URLs, v1/v2+ boundary.
- `docs/playground/README.md` — surface table updated with privacy/telemetry row (via `generate-docs.py`).
- `bash scripts/run/verify-harness.sh --quick` exits 0 (19/19 checks pass).

## Summary

playground で何を収集し何を収集しないか、エラー報告をどう扱うかを定義し、
設定またはコードレベルの guardrail まで入れる。単なる方針文ではなく、実装
に反映される形にする。

## Current state

- playground はこれから作るため、privacy/telemetry が後追いになりやすい。
- share link や examples 利用だけでもログ方針が必要。
- error reporting を入れる場合も opt-in/opt-out を設計する必要がある。

## Acceptance

- [x] privacy / telemetry 方針が文書化される。
- [x] 必要なら telemetry の on/off 設定や無効化既定が実装される。
- [x] error reporting の送信条件がコードまたは config に反映される。
- [x] docs に利用者向け説明が追加される。

## References

- `playground/src/telemetry.ts` — code-level guardrail (new)
- `docs/playground/privacy-telemetry-policy.md` — full policy
- `docs/adr/ADR-017-playground-execution-model.md`
- `docs/adr/ADR-021-playground-share-url-format.md`
- `docs/adr/ADR-022-playground-deployment-and-caching.md`