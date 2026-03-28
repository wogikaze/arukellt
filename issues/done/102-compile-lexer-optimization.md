# コンパイル速度: Lexer / Parser のホットパス最適化

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-03-28
**ID**: 102
**Depends on**: 100
**Track**: compile-speed
**Blocks v4 exit**: no

## Summary

`--time` フラグで計測した後に、lex/parse フェーズが律速している場合のための最適化 issue。
大規模ファイル (500行以上) での lexer のバイト処理速度を改善する。

## 対象最適化

- 文字列スキャンの SIMD 化 (memchr crate による改行/クォート検索)
- トークン種別のテーブルルックアップ最適化 (ASCII 範囲の高速分岐)
- Parser のバックトラック削減 (LL(1) 変換の徹底)

## 受け入れ条件

1. `--time` で lex + parse が `hello.ark` で 5ms 以内であることを確認
2. `parser.ark` (500行) で lex + parse が 50ms 以内
3. `memchr` crate の活用で文字列スキャンを高速化

## 参照

- roadmap-v4.md §2 (コンパイル時間目標)
