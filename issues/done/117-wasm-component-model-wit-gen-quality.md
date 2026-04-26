# Component Model: WIT 生成品質の向上と往復検証

**Status**: done
**Created**: 2026-03-28
**Updated**: 2026-04-15
**ID**: 117
**Depends on**: —
**Track**: wasm-quality
**Blocks v4 exit**: no

---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: done` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/117-wasm-component-model-wit-gen-quality.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

`ark-wasm/src/component/mod.rs` の `mir_to_wit_world()` が生成する WIT ソースの
品質を向上させる。現在は struct を `record` に、enum を `variant` に変換しているが、
WIT の `resource`・`option<T>`・`result<T, E>`・`tuple<T1, T2>` 型への変換も行う。
生成 WIT が `wasm-tools component wit` で正しくパースできることをテストに追加する。

## 受け入れ条件

1. `Option<T>` → WIT `option<T>` に変換
2. `Result<T, E>` → WIT `result<T, E>` に変換
3. タプル型 → WIT `tuple<T1, T2>` に変換
4. 生成 WIT を `wasm-tools component wit` でパースできることを CI に追加
5. kebab-case 変換の一貫性確認 (`camelCase` / `snake_case` → `kebab-case`)

## 参照

- `docs/spec/spec-WASI-0.2.10/OVERVIEW.md` §WIT形式の読み方
- `crates/ark-wasm/src/component/mod.rs`
