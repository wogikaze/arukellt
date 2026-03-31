# Stdlib Docs: examples を compile-check して docs drift を防ぐ

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 401
**Depends on**: 398
**Track**: stdlib-docs
**Blocks v1 exit**: no
**Priority**: 7

## Summary

docs に掲載する code snippet や recipe snippet が実際に通ることを CI で保証する。display 用 snippet と実行用 fixture が別れている場合でも、最低限の compile-check または extraction を通す。

## Current state

- docs code snippet が実装変更で古くなっても、CI がすぐには検知できない。
- recipe は fixture に近いが、module page の inline snippet は検証されにくい。
- docs drift が発生すると stdlib surface の信頼性が落ちる。

## Acceptance

- [ ] docs snippet を抽出して compile-check する仕組みが追加される。
- [ ] recipe / example / inline snippet の検証範囲が定義される。
- [ ] CI が docs snippet の壊れを検出する。
- [ ] 失敗時にどの snippet が壊れたかを出力する。

## References

- ``docs/stdlib/**``
- ``tests/fixtures/``
- ``scripts/check-docs-consistency.py``
