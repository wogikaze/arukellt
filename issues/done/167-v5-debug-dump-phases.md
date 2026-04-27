---
Status: done
Updated: 2026-03-30
ID: 167
Track: main
Depends on: 163, 164, 165
Orchestration class: implementation-ready
Blocks v1 exit: no
---
# v5 Debug dump phases for the selfhost compiler

## Summary

Arukellt 版コンパイラに Rust 版 `ARUKELLT_DUMP_PHASES` 相当のデバッグ出力を実装する。Phase 1 では tokens / ast、Phase 2 以降で hir / mir、Phase 3 で wasm まで広げる。

## Acceptance

- [x] `tokens`, `ast`, `hir`, `mir`, `wasm` の各 dump 対象が issue 本文で明示されている
- [x] debug dump は stderr に出力し、binary/stdout 出力責務と衝突しない
- [x] 出力フォーマット一致の検証導線が bootstrap verification 側から参照できる

## Implementation tasks

1. CLI / env surface を selfhost driver に追加する
2. 各フェーズの text dump を段階的に接続する
3. Rust 版との比較用途に耐える出力フォーマットへ寄せる

## References

- `issues/open/163-v5-phase1-driver-cli.md`
- `issues/open/166-v5-bootstrap-verification.md`
- `crates/arukellt/src/commands.rs`