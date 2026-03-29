# ADR-011 に沿った `std::host` layer の段階的ロールアウト

**Status**: open
**Created**: 2026-03-29
**Updated**: 2026-03-29
**ID**: 136
**Depends on**: 137, 138, 077, 139
**Track**: wasi-feature
**Blocks v1 exit**: no

## Summary

ADR-011 で決定した `std::host::*` layering を、stdlib・compiler・docs・verification に反映する。
目的は host capability を `std::*` 直下から切り離し、
pure stdlib と explicit host facade を明確に分離すること。

## 受け入れ条件

1. `std::host::*` の naming / target policy が `std/manifest.toml`、generated docs、issue queue で一貫する
2. `137`, `138`, `077`, `139` が完了し、依存グラフ上の残課題がなくなる
3. `scripts/verify-harness.sh --quick` が status 0
4. child issue で追加した T1/T3 実行確認手順が docs から辿れる

## 参照

- `docs/adr/ADR-011-wasi-host-layering.md`
- `docs/stdlib/std.md`
- `docs/migration/t1-to-t3.md`
