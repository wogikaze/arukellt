# v5 Parser: expression / declaration parsing

**Status**: open
**Created**: 2026-03-29
**ID**: 173
**Depends on**: 172
**Track**: main
**Blocks v1 exit**: no

## Summary

Pratt parsing を中心に、式・宣言・文・型注釈の構文解析本体を実装する。Phase 1 で頻出構文をカバーし、selfhost driver から AST を生成できる状態を作る。

## Acceptance

- [ ] 二項演算、前置演算、呼び出し、フィールドアクセスなど主要 expression parser が揃う
- [ ] let / fn / struct / enum / if / while / return / import など主要 declaration / statement parser が揃う
- [ ] 型注釈と generic-looking surface を parser が解釈できる

## References

- `issues/open/172-v5-parser-ast-surface.md`
- `crates/ark-parser/src/parser/expr.rs`
- `crates/ark-parser/src/parser/decl.rs`
- `crates/ark-parser/src/parser/stmt.rs`
- `crates/ark-parser/src/parser/ty.rs`
