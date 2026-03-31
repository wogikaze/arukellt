# Stdlib: test モジュールの assertion / snapshot helper を拡充する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 390
**Depends on**: —
**Track**: stdlib-api
**Blocks v1 exit**: no
**Priority**: 8

## Summary

`std::test` family を fixture 実行の裏方ではなく、ユーザーが使う test support API として育てる。assert、diff 表示、失敗メッセージ、snapshot 風 helper などを整え、playground examples や docs recipes にも使えるようにする。

## Current state

- test support は最低限で、利用者向け API としてのまとまりが弱い。
- 失敗時のメッセージ整形や比較支援が薄い。
- docs 側に test family を使う recipes が少ない。

## Acceptance

- [ ] assertion helper が追加または整理される。
- [ ] 少なくとも 1 つの snapshot 風比較 helper が提供される。
- [ ] 失敗メッセージの表示を確認する fixture が追加される。
- [ ] docs / cookbook で test helper を使う例が増える。

## References

- ``std/test/**``
- ``tests/fixtures/``
- ``docs/stdlib/cookbook.md``
