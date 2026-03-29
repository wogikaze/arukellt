# v5 Driver/CLI: command surface and exit behavior

**Status**: open
**Created**: 2026-03-29
**ID**: 175
**Depends on**: 162
**Track**: main
**Blocks v1 exit**: no

## Summary

selfhost compiler の CLI entrypoint を整理し、`parse` / `compile` の surface、引数処理、exit code 契約を定義する。debug dumping は #167 で追う。

## Acceptance

- [ ] `parse` / `compile` などの command surface が定義されている
- [ ] 引数解釈と usage / failure path の責務が明確になっている
- [ ] 正常系 0 / 失敗系 1 の exit behavior を追跡できる

## References

- `issues/open/162-v5-phase1-parser.md`
- `crates/arukellt/src/main.rs`
- `crates/arukellt/src/commands.rs`
