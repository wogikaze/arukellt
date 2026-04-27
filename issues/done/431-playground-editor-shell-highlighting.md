---
Status: done
Created: 2026-03-31
Updated: 2026-04-03
ID: 431
Track: playground
Depends on: 429
Orchestration class: implementation-ready
Blocks v1 exit: False
Priority: 4
# Playground: editor shell と syntax highlighting を実装する
---
# Playground: editor shell と syntax highlighting を実装する

## Status note (2026-04-03)

この issue は historical implementation-parts work としては done のまま保持するが、**current repo で browser-reachable playground が存在する証拠としては使わない**。

- done の範囲は editor shell / highlighting / marker surface の component-level work に限定する。
- mount 済み browser entrypoint、repo-visible route、publish proof、docs/extension alignment はこの issue の完了証拠ではない。
- current product-proof tracking は `issues/open/465-playground-false-done-audit-and-status-rollback.md` と `issues/open/466`〜`472` で行う。

## Summary

Monaco または CodeMirror ベースの editor shell を作り、既存 syntax 資産を反映する。editor は playground の顔なので、最低限の highlighting、diagnostics markers、format action を揃える。

## Current state

- docs site は static shell のみで editor UI がない。
- syntax highlighting の source of truth は extension 側にあるが、browser 用再利用が未整理。
- editor と Wasm engine の接続点が無い。

## Acceptance

- [x] browser editor が追加される。
- [x] syntax highlighting が動作する。
- [x] diagnostics markers または panel 表示が動作する。
- [x] format action が editor から呼べる。

## References

- ``docs/index.html``
- ``extensions/arukellt-all-in-one/**``
- ``crates/ark-parser/src/fmt.rs``