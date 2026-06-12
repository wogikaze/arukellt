---
Status: open
Created: 2026-03-31
Updated: 2026-06-12
ID: 443
Track: component-composition
Depends on: 442, 476
Orchestration class: blocked-by-upstream
Blocks v1 exit: False
Priority: 5
Blocked by: "#476 wasm-tools compose integration"
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
- linking / composition 機構なし。
- package-level component 概念なし。

## Acceptance

- [ ] component 同士の import/export を解決可能にする。
- [ ] 複数 component を 1つの実行単位に合成できる。
- [ ] dependency graph が構築される。
- [ ] conflict（名前/型）の検出が可能。
- [ ] CLI から compose/build が実行可能。

## References

- `crates/ark-driver/`
- `crates/ark-wasm/`
- `docs/target-contract.md`
- `docs/ark-toml.md`
