---
Status: done
Created: 2026-03-31
Updated: 2026-03-31
ID: 444
Track: editor-runtime
Depends on: 439, 440, 441, 443
Orchestration class: implementation-ready
Blocks v1 exit: False
Priority: 6
# VSCode Extension × Component: Playground / Editor から component を直接生成・実行できる導線を作る
---
# VSCode Extension × Component: Playground / Editor から component を直接生成・実行できる導線を作る

## Summary

VSCode拡張から直接 component を生成・実行・検証できる導線を作る。これにより editor → build → run → inspect のループを一体化する。

## Current state

- VSCode は LSP + basic commands のみ。
- component は CLI 前提。
- editor と runtime が分断されている。

## Acceptance

- [x] VSCode から component build が実行可能。
- [x] build 結果（WIT / wasm / component）を表示できる。
- [x] run / test が editor から可能。
- [x] errors が editor diagnostics に反映される。
- [x] playground / preview との連携が可能。

## References

- `extensions/arukellt-all-in-one/`
- `crates/arukellt/`
- `crates/ark-wasm/`
- `docs/examples/`