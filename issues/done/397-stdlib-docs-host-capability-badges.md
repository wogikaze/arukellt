# Stdlib Docs: host API の target / capability / warning 表示を統一する

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 397
**Depends on**: 364
**Track**: stdlib-docs
**Blocks v1 exit**: no
**Priority**: 3

## Summary

host family の docs で、target 制約、必要 capability、未実装状態、stub / experimental を同じ見た目で表示する。利用者がページを開いた瞬間に「今使えるか」を判断できる状態にする。

## Current state

- host module の情報は manifest にあるが、page 上での見え方が統一されていない。
- warning、badge、callout の使い分けがページごとにぶれやすい。
- stub や target 限定 API が reference 一覧の中で埋もれやすい。

## Acceptance

- [x] host API の表示フォーマットが統一される。
- [x] target / capability / status が page 上で一目で分かる。
- [x] generator が metadata から自動表示する。
- [x] 少なくとも 1 つの snapshot 的 docs test が追加される。

## References

- ``std/manifest.toml``
- ``docs/stdlib/modules/*.md``
- ``docs/stdlib/reference.md``
- ``scripts/generate-docs.py``
