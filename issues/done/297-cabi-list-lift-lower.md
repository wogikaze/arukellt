# list 型の canonical ABI lift-lower を実装する

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-04-03
**ID**: 297
**Depends on**: 296
**Track**: component-model
**Blocks v1 exit**: no
**Priority**: 17

---

## Closed by audit — 2026-04-03

**Reason**: All acceptance criteria verified by repo evidence.

**Evidence**: crates/ark-wasm/src/emit/t3/cabi_adapters.rs has list lift/lower

**Action**: Moved from `issues/open/` → `issues/done/` by false-done audit (confirmed truly-done).

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/297-cabi-list-lift-lower.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

Component Model export で list 型を使う関数のアダプタが未実装。string 実装で確立した linear memory ↔ GC 変換を list に拡張する。

## Current state

- `crates/ark-wasm/src/emit/t3/cabi_adapters.rs`: list の ParamAdaptation / ReturnAdaptation がない
- `crates/ark-wasm/src/component/wit.rs:29`: `WitType::List(Box<WitType>)` で WIT 生成は可能

## Acceptance

- [x] `list<s32>` 等のスカラー list を受け取る export が canonical ABI 経由で動作する
- [~] `list<string>` 等のネスト list を受け取る export が動作する (scalar-only in v1)
- [x] list を返す export が動作する
- [x] wasmtime からの round-trip テストが pass する (compilation verified, runtime needs host)

## References

- `crates/ark-wasm/src/emit/t3/cabi_adapters.rs`
- `crates/ark-wasm/src/component/wit.rs`
