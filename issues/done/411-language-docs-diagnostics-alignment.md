---
Status: done
Created: 2026-03-31
Updated: 2026-03-31
ID: 411
Track: language-docs
Depends on: —
Orchestration class: implementation-ready
---
# Language Docs: diagnostics / error code 文書を実装と揃える
**Blocks v1 exit**: no
**Priority**: 6

## Summary

error code、diagnostic severity、出力形式の文書を実装に寄せて整える。これは単なる文章修正ではなく、error-code 一覧と compiler / lint / LSP の出力を同じ分類で説明できるようにする作業。

## Current state

- diagnostics 関連 docs は存在するが、error code・severity・出力経路の説明が複数箇所に分かれている。
- linter 導入や selfhost parity により、説明すべき出力契約が増えている。
- 実装側のコード一覧と docs の対応が弱い。

## Acceptance

- [x] diagnostics と error codes の対応表が更新される。
- [x] compiler / lint / LSP の出力区分が docs に反映される。
- [x] 関連 docs が相互リンクで整理される。
- [x] 少なくとも主要 error code 系 docs に整合チェックが入る。

## References

- ``docs/compiler/diagnostics.md``
- ``docs/compiler/error-codes.md``
- ``crates/ark-diagnostics/src/codes.rs``