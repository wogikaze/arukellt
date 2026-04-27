---
Status: done
Created: 2026-03-31
Updated: 2026-03-31
ID: 381
Track: playground
Depends on: 380
Orchestration class: implementation-ready
Blocks v1 exit: False
Priority: 24
# Playground: examples loader と share link を実装する
---
# Playground: examples loader と share link を実装する

## Summary

stdlib / docs の example コードを playground に読み込む examples loader と、現在のエディタ内容を URL として共有できる share link 機能を実装する。examples は `tests/fixtures/` や `docs/examples/` を source of truth とし、二重管理しない。

## Current state

- `tests/fixtures/` に 588 fixture (example として利用可能)
- `docs/examples/` に example コードあり
- `docs/stdlib/cookbook.md` に recipe あり
- playground 用の examples manifest なし
- share link (URL encoding / hash) 機能なし

## Acceptance

- [x] examples ドロップダウンから fixture / example を選択して editor に読み込める
- [x] examples の source of truth が `tests/fixtures/` or `docs/examples/` と一致する
- [x] share link がエディタ内容を URL hash にエンコードする
- [x] share link から playground を開くと同じコードが復元される
- [x] examples が host capability 必須かどうかを明示する

## References

- `tests/fixtures/` — fixture files
- `docs/examples/` — example code
- `docs/stdlib/cookbook.md` — cookbook recipes