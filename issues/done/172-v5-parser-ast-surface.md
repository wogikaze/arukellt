---
Status: done
Created: 2026-03-29
Updated: 2026-03-30
ID: 172
Track: main
Depends on: 161
Orchestration class: implementation-ready
Blocks v1 exit: False
# v5 Parser: AST / span / import surface
---
# v5 Parser: AST / span / import surface

## Summary

selfhost parser の前提になる AST / Span / ParseError / import surface を整理する。Pratt parser 本体に入る前に、Rust 版と対応づけられる構文木 surface を確定させる。

## Acceptance

- [x] Expr / Stmt / Pat / Type / Decl など parser が返す主要ノード群が定義されている
- [x] Span / location / parse error surface が parser 実装に使える形で定義されている
- [x] import / module / top-level declaration の表現が selfhost parser で扱える

## References

- `issues/done/161-v5-phase1-lexer.md`
- `crates/ark-parser/src/parser/`