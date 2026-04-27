---
Status: done
Created: 2026-03-31
Updated: 2026-04-03
ID: 298
Track: component-model
Depends on: 296
Orchestration class: implementation-ready
---
# option / result 型の canonical ABI lift-lower を実装する
**Blocks v1 exit**: no
**Priority**: 19

---

## Closed by audit — 2026-04-03

**Reason**: All acceptance criteria verified by repo evidence.

**Evidence**: cabi_adapters.rs has OptionType and ResultType lift/lower at lines 58-72

**Action**: Moved from `issues/open/` → `issues/done/` by false-done audit (confirmed truly-done).

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/298-cabi-option-result-lift-lower.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

`option<T>` と `result<T, E>` の canonical ABI アダプタが未実装。WIT 生成は可能だが lift-lower コードがない。

## Current state

- `crates/ark-wasm/src/component/wit.rs:30-34`: `WitType::Option` / `WitType::Result` が定義済み
- `crates/ark-wasm/src/emit/t3/cabi_adapters.rs`: option / result のアダプタなし
- WIT canonical ABI 上、option は discriminant + payload、result は ok/err discriminant + payload

## Acceptance

- [x] `option<s32>` を受け取る/返す export が動作する
- [x] `result<s32, string>` を受け取る/返す export が動作する
- [x] wasmtime からの round-trip テストが pass する

## References

- `crates/ark-wasm/src/emit/t3/cabi_adapters.rs`
- `crates/ark-wasm/src/component/wit.rs`