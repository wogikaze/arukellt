# Language Docs: transitional / archive banner を一括適用する

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 414
**Depends on**: 372
**Track**: language-docs
**Blocks v1 exit**: no
**Priority**: 9

## Summary

preview や historical な文書が current docs と同列に見えないよう、banner と一覧表示を整える。これは境界方針を実際のページへ適用する rollout issue。

## Current state

- `syntax-v1-preview.md` のような文書が current doc と見分けにくい。
- archive directory にない historical 文書も存在する。
- banner の書き方や場所が統一されていない。

## Acceptance

- [x] transitional / archive 対象ページに banner が入る。
- [x] README の一覧にもラベルが出る。
- [x] 少なくとも既知の preview / historical ページが全て対象化される。
- [x] banner 文言が統一される。

## References

- ``docs/language/syntax-v1-preview.md``
- ``docs/language/**``
- ``docs/spec/**``
- ``scripts/generate-docs.py``
