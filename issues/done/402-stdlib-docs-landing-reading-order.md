# Stdlib Docs: landing page と読む順番を family 単位で再設計する

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 402
**Depends on**: 396
**Track**: stdlib-docs
**Blocks v1 exit**: no
**Priority**: 8

## Summary

README と landing page を単なる一覧ではなく、目的別の reading order に変える。初学者・実務利用者・設計確認者・移行中ユーザーで読む入口を分ける。

## Current state

- `docs/stdlib/README.md` は資料一覧としては十分だが、読む順番のガイドは弱い。
- legacy landing pages と current pages が並んで見えることがある。
- module family ごとの優先ドキュメントが整理されていない。

## Acceptance

- [x] 目的別の reading order が README に追加される。
- [x] family landing page から適切な overview / recipe / reference へ誘導する。
- [x] legacy page の導線が current first になる。
- [x] README の自動生成または更新手順が明文化される。

## References

- ``docs/stdlib/README.md``
- ``docs/stdlib/modules/*.md``
- ``docs/README.md``
