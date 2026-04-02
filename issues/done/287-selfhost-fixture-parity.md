# セルフホスト fixture parity テストを構築する

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 287
**Depends on**: 290
**Track**: selfhost
**Blocks v1 exit**: no
**Priority**: 7

## Summary

セルフホストコンパイラが Rust コンパイラと同じ出力を生成するか、代表 fixture で検証する仕組みがない。fixpoint は「同じバイナリを生成する」だが、fixture parity は「同じプログラムを正しくコンパイルできる」。

## Current state

- `scripts/verify-bootstrap.sh:282`: `fixture-parity: not-verified` と出力するだけ
- セルフホストで fixture をコンパイル＆実行するスクリプトがない

## Acceptance

- [x] 代表 fixture (少なくとも 50 個) を selfhost コンパイラでコンパイル＆実行するスクリプトが存在する
- [x] 比較基準は「実行結果 (stdout) の一致」とする（バイナリ同一性は求めない）
- [x] 不一致箇所のリストが出力される
- [x] `verify-bootstrap.sh --fixture-parity` で呼び出し可能

## References

- `scripts/verify-bootstrap.sh`
- `tests/fixtures/`
- `src/compiler/driver.ark`
