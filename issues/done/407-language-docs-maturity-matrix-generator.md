# Language Docs: feature maturity matrix の生成元と更新フローを実装する

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 407
**Depends on**: 369
**Track**: language-docs
**Blocks v1 exit**: no
**Priority**: 2

## Summary

maturity matrix を手編集ページではなく、spec 内のラベルや機能一覧から再生成できる形にする。そうすることで current-state と docs のドリフトを減らす。

## Current state

- feature maturity は spec の各章に埋め込まれており、一覧で把握しにくい。
- matrix を手で保守すると drift しやすい。
- どの feature が stable / experimental / unimplemented かを CI が見ていない。

## Acceptance

- [x] matrix の生成元データが定義される。
- [x] matrix 文書が自動生成または半自動更新される。
- [x] 更新手順が文書化される。
- [x] ラベル不整合を検出するチェックが追加される。

## References

- ``docs/language/spec.md``
- ``docs/language/README.md``
- ``scripts/gen/generate-docs.py``
