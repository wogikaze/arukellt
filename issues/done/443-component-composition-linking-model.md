---
Status: done
Created: 2026-03-31
Updated: 2026-06-15
ID: 443
Track: component-composition
Depends on: 442, 476
Orchestration class: implementation-ready
Blocks v1 exit: False
Priority: 5
Status note: Closed 2026-06-15 — Phase 3 `wac plug` delegation via `arukellt compose` + selfhost wrapper; gate `gate-443-component-composition-phase3.py`.
Close gate: gate_443 → scripts/check/gate-443-component-composition-phase3.py
---
## Reopened by audit — 2026-06-12 (Slice C)

**Reopen reason:** All acceptance items checked but `arukellt compose` returns `CMD_NOT_YET()` in `src/compiler/main/commands.ark`; no linking model, dependency graph builder, or compose CLI exists in the selfhost path.

**Violated acceptance:** All five acceptance items (import/export resolution, multi-component synthesis, dependency graph, conflict detection, CLI compose/build)

**Evidence files:**
- `src/compiler/main/commands.ark` (`parse_reserved_command` → `CMD_NOT_YET` for `compose`)
- `issues/open/476-wasm-tools-compose-integration.md` (active track for compose)

**Follow-up split issue:** none (scope covered by #476)

---

# Component Composition: 複数コンポーネントの合成と linking モデルを定義・実装する

## Summary

複数の Wasm Component を合成し、依存関係を解決して実行可能な構成を作る linking モデルを導入する。package system / dependency graph と連動させる。

## Acceptance (#443)

- [x] component 同士の import/export を解決可能にする（WIT sidecar 型マッチング — Phase 2; バイナリ抽出は未実装）。
- [x] 複数 component を 1つの実行単位に合成できる（恒久 `wac` 委譲 — Phase 3; `arukellt compose` + `scripts/run/arukellt-selfhost.sh`）。
- [x] dependency graph が構築される（scaffold: テキスト出力）。
- [x] conflict（名前/型）の検出が可能（パス衝突 + WIT import/export 不一致）。
- [x] CLI から compose/build が実行可能（`--validate` + `wac plug` 委譲）。

## References

- `src/compiler/main/compose_cmd.ark`
- `scripts/run/arukellt-selfhost.sh`
- `docs/adr/ADR-034-component-composition-linking.md`
- `scripts/check/gate-443-component-composition-phase3.py`
