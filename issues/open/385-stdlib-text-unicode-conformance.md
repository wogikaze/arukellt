# Stdlib: text モジュールの Unicode / UTF-8 契約を fixture で固定する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 385
**Depends on**: —
**Track**: stdlib-api
**Blocks v1 exit**: no
**Priority**: 3

## Summary

文字列 API の仕様を曖昧な方針文ではなく実行テストで固定する。byte length、scalar count、slice 境界、case conversion、invalid UTF-8 取り扱いなど、text family のふるまいを明示し、後続の docs と LSP hover の説明根拠にする。

## Current state

- text 系 API は存在するが、Unicode scalar / byte / grapheme のどれを基準にするかが API ごとに読み取りづらい。
- UTF-8 境界や invalid sequence に関する回帰テストがまとまっていない。
- docs 上の説明と実装テストが 1 対 1 で結びついていない。

## Acceptance

- [ ] Unicode / UTF-8 まわりの期待動作を示す fixture 群が追加される。
- [ ] byte length と scalar length の違いを確認するテストがある。
- [ ] invalid UTF-8 や境界エラーのふるまいがテストで固定される。
- [ ] 関連 docs が fixture を参照する。

## References

- ``std/text/**``
- ``tests/fixtures/``
- ``docs/stdlib/reference.md``
- ``docs/stdlib/cookbook.md``
