# Linter: ark.toml で lint 設定を管理する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 351
**Depends on**: 350
**Track**: linter
**Blocks v1 exit**: no
**Priority**: 20

## Summary

`ark.toml` に `[lint]` セクションを追加し、project ごとに lint rule の severity を設定できるようにする。allow / warn / deny の 3 段階と、rule 単位の設定を持たせる。

## Current state

- `crates/ark-manifest/src/lib.rs`: `[lint]` セクションの定義なし
- `docs/ark-toml.md`: lint 設定の記述なし
- `ark.toml` のスキーマに lint 関連フィールドなし

## Acceptance

- [ ] `ark.toml` で `[lint]` セクションが parse される
- [ ] `[lint]` で `allow = ["rule-name"]` / `warn = ["rule-name"]` / `deny = ["rule-name"]` が設定可能
- [ ] CLI / LSP が `ark.toml` の lint 設定を尊重する
- [ ] `docs/ark-toml.md` に `[lint]` セクションが文書化される

## References

- `crates/ark-manifest/src/lib.rs` — manifest 定義
- `docs/ark-toml.md` — ark.toml documentation
