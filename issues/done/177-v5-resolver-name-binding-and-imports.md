---
Status: done
Created: 2026-03-29
Updated: 2026-03-30
ID: 177
Track: main
Depends on: 163
Orchestration class: implementation-ready
Blocks v1 exit: False
# v5 Resolver: name binding and imports
---
# v5 Resolver: name binding and imports

## Summary

selfhost resolver で scope stack、symbol binding、import 解決を実装する。Phase 2 のうち semantic name resolution に絞った子 issue。

## Acceptance

- [x] local / function / type / import symbol の binding ルールが追跡できる
- [x] 未定義名と重複定義の diagnostics 導線がある
- [x] typed HIR 前段として必要な解決済み情報が揃う

## References

- `issues/open/163-v5-phase1-driver-cli.md`
- `crates/ark-resolve/src/bind.rs`