# ツール: Wasm バイナリサイズ内訳分析ツール

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-03-28
**ID**: 111
**Depends on**: —
**Track**: benchmark
**Blocks v4 exit**: no

## Summary

生成した `.wasm` ファイルのサイズ内訳を
セクション別・関数別・型定義別に可視化するツールを追加する。
`wasm-objdump` の出力をパースして Arukellt ソースの関数名とマッピングする。

## 受け入れ条件

1. `arukellt analyze --wasm-size output.wasm` コマンドで内訳表示
2. セクション別 (type, import, function, memory, global, export, data, code) のサイズ表示
3. 上位10関数のコードサイズランキング
4. 出力例:

   ```
   code section: 4.2KB (73%)
     fib:           128B
     print_i64:     512B
     ...
   type section: 0.8KB (14%)
   data section: 0.3KB (5%)
   ```

## 参照

- roadmap-v4.md §11 (docs/process/benchmark-results.md)
