# CLI: --time フラグ + フェーズ別コンパイル時間計測

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-03-28
**ID**: 100
**Depends on**: —
**Track**: compile-speed
**Blocks v4 exit**: yes

## Summary

`arukellt compile --time` フラグで各コンパイルフェーズの時間を stderr に出力する。
roadmap-v4.md §6 item 6 で明示的に要求されているフラグ。
律速ステップを特定してから最適化するための計測インフラとして必須。

## 出力フォーマット

```
[arukellt] lex:        1.2ms
[arukellt] parse:      4.5ms
[arukellt] resolve:    8.3ms
[arukellt] typecheck: 12.1ms
[arukellt] lower:      3.4ms
[arukellt] opt:        2.1ms  (passes: const_fold=5, dce=3, ...)
[arukellt] emit:      18.7ms
[arukellt] total:     50.3ms
```

## 受け入れ条件

1. `crates/arukellt/src/main.rs` に `--time` フラグ追加
2. `ark-driver/src/session.rs` の各フェーズ呼び出しを `Instant::now()` で囲む
3. `--time` フラグがなければオーバーヘッドゼロ (条件分岐のみ)
4. opt フェーズはパス別の適用回数も表示

## 参照

- roadmap-v4.md §6 item 6
