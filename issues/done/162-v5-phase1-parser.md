---
Status: done
Updated: 2026-03-30
ID: 162
Track: main
Depends on: 172, 173, 174
Orchestration class: implementation-ready
Blocks v1 exit: False
Status note: Parent issue for v5 parser work. Close when the child issues are complete.
# v5 Phase 1: Parser epic
---
# v5 Phase 1: Parser epic


## Summary

v5 の parser 作業は、AST surface 定義・構文解析本体・fixture parity / diagnostics で性質が分かれる。1 本の issue だと進捗と依存が見えにくいため、この issue は parser 系の完了マーカーとして扱う。

## Acceptance

- [x] #172, #173, #174 が完了している
- [x] v5 parser の残課題が child issue に分解され、親 issue 本文に未整理の実装責務が残っていない
- [x] selfhost parser の完了条件が issue queue 上で追跡できる

## References

- `issues/done/159-v5-language-spec-freeze.md`
- `issues/done/160-v5-selfhost-stdlib-checklist.md`
- `issues/done/161-v5-phase1-lexer.md`
- `crates/ark-parser/src/parser/`