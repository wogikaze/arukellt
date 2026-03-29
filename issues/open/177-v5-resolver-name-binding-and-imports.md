# v5 Resolver: name binding and imports

**Status**: open
**Created**: 2026-03-29
**ID**: 177
**Depends on**: 163
**Track**: main
**Blocks v1 exit**: no

## Summary

selfhost resolver で scope stack、symbol binding、import 解決を実装する。Phase 2 のうち semantic name resolution に絞った子 issue。

## Acceptance

- [ ] local / function / type / import symbol の binding ルールが追跡できる
- [ ] 未定義名と重複定義の diagnostics 導線がある
- [ ] typed HIR 前段として必要な解決済み情報が揃う

## References

- `issues/open/163-v5-phase1-driver-cli.md`
- `crates/ark-resolve/src/bind.rs`
