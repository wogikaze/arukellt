# v5 Bootstrap guide documentation

**Status**: done
**Updated**: 2026-03-30
**ID**: 169
**Depends on**: 166, 167
**Track**: main
**Blocks v1 exit**: no

## Summary

`docs/compiler/bootstrap.md` に、Stage 0/1/2 のブートストラップ手順と fixpoint 未達時のデバッグ導線をまとめる。検証スクリプト本体は #166、dump phases は #167 に依存する。

## Acceptance

- [x] bootstrap guide が Stage 0/1/2、前提ツール、fixpoint 検証、debug 手順を含む
- [x] bootstrap verification (#166) と dump phases (#167) への参照が docs から辿れる
- [x] docs 単体で第三者が再現手順を追える構成になっている

## References

- `issues/open/166-v5-bootstrap-verification.md`
- `issues/open/167-v5-debug-dump-phases.md`
- `docs/current-state.md`
