# v5 Parser: fixture parity and diagnostics

**Status**: open
**Created**: 2026-03-29
**ID**: 174
**Depends on**: 173
**Track**: main
**Blocks v1 exit**: no

## Summary

selfhost parser の完成条件を、fixture parity と syntax diagnostics で詰める。構文木の shape が揃っていても、差分確認手段がないと queue 上で完了判定しにくいため独立 issue とする。

## Acceptance

- [ ] parser fixture で Rust 版との差分確認手段がある
- [ ] syntax error 時の報告 surface が定義されている
- [ ] parser 完了判定が "実装済み" ではなく parity / diagnostics ベースで追跡できる

## References

- `issues/open/173-v5-parser-expression-and-decl-parsing.md`
- `tests/fixtures/`
- `crates/ark-diagnostics/src/helpers.rs`
