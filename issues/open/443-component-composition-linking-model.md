---
Status: open
Created: 2026-03-31
Updated: 2026-06-15
ID: 443
Track: component-composition
Depends on: 442, 476
Orchestration class: implementation-ready
Blocks v1 exit: False
Priority: 5
Status note: Phase 1 scaffold (2026-06-15) — `arukellt compose` validates plug plans and prints dependency graph; native linking deferred. See `docs/adr/ADR-034-component-composition-linking.md`.
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

## Current state

- 単体 component 出力のみ。
- `arukellt compose` Phase 1 scaffold: パス検証 + dependency graph 出力 + `wac plug` 委譲ヒント（`src/compiler/main/compose_cmd.ark`）。
- WIT import/export 型マッチングとネイティブ合成は未実装。
- package-level component 概念なし。

## Remaining acceptance (#443)

Phase 1 landed 2026-06-15. Still open:

- [ ] component 同士の import/export を解決可能にする（WIT 型マッチング — Phase 2）。
- [ ] 複数 component を 1つの実行単位に合成できる（in-tree または恒久 `wac` 委譲の契約化 — Phase 3）。
- [x] dependency graph が構築される（scaffold: テキスト出力）。
- [x] conflict（名前/型）の検出が可能（scaffold: 同一パス衝突のみ）。
- [x] CLI から compose/build が実行可能（scaffold: `--validate` + delegate hint; バイナリ合成は `wac` 委譲）。

## References

- `crates/ark-driver/`
- `crates/ark-wasm/`
- `docs/target-contract.md`
- `docs/ark-toml.md`
