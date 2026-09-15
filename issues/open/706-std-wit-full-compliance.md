---
Status: open
Created: 2026-07-14
Updated: 2026-09-15
ID: 706
Track: stdlib
Depends on: 606
Priority: 1
Source: 2026-09-15 verification audit of the merged gc-host snapshot
Action: Moved from `issues/done/` to `issues/open/` by verification audit (2026-09-15).
---

# 706 — std::wit Full WIT 1.0 Compliance

## Audit resolution — 2026-09-15

この issue は、実装と close gate の契約が一致していないため `issues/done/` から
`issues/open/` へ戻した。

`std::wit::parser::parse_full` と `std::wit::ast` の full-parser surface 自体は存在する。
しかし、現在の compiler import loader は `component::wit_parse_text::parse_wit_import_text` を
直接呼んでおり、`parse_full` を呼んでいない。過去に `parse_full` を呼ぶ修正を試したが、
現行の bootstrap compiler では `WitNode` の enum result を扱う箇所で runtime trap になり、
WIT import fixture が実行できなかった。そのため、二重 parse を残したまま gate の文字列だけを
満たす変更は採用しない。

現在の close gate は issue が open であることを明示して `SKIP` する。bootstrap が
`parse_full` の result を安全に受け取り、compiler metadata へ一度だけ変換できるように
なった後、実装と gate を再び同時に閉じる。

## 現在の受け入れ状況

- [x] Full WIT 1.0 syntax parser が `std::wit::ast` に存在し、`parse_full` から利用できる。
- [x] Shared kebab/snake/Pascal helpers と AST→WIT type surface が `std::wit` に存在する。
- [x] Compiler の重複した `wit_parse_types.ark` model は削除されている。
- [ ] Compiler WIT import が `parse_full` を一回だけ呼び、その AST を resolver/codegen 用 metadata へ変換する。
- [ ] Compiler-local text parser が syntax parser の owner ではなく metadata adapter になる。
- [ ] Full compliance close gate と WIT import fixture parity が同時に通る。

## 再開時の完了条件

1. `src/compiler/component/wit_parse_import.ark` が `std::wit::parser::parse_full` を一回だけ実行する。
2. `WitNode` の package/interface/function/record/enum/variant 情報を compiler metadata へ変換し、
   source text をもう一度 parser に渡さない。
3. named type、component import の diagnostics、WIT binding の既存 fixture を維持する。
4. bootstrap compiler で WIT import fixture を compile/validate/execute する。
5. `python3 scripts/check/gate-706-std-wit-full-compliance.py` と
   `python3 scripts/check/gate-component-wit-productization.py` が PASS する。

## Verification evidence

- `python3 scripts/check/gate-706-std-wit-full-compliance.py` は、現状では `#706 open` の明示的な
  `SKIP` を返す。
- `git show 139e586e6` に、`parse_full` 呼び出しを削除した理由と、現行 bootstrap での trap が記録されている。
- `python3 scripts/check/gate-665-wit-import-compose-roundtrip-e2e.py` は、現在の compiler-local
  metadata path を使う限り WIT compose regression を検査する。

## References

- `std/wit/ast.ark`
- `std/wit/parser.ark`
- `src/compiler/component/wit_parse_import.ark`
- `src/compiler/component/wit_parse_text.ark`
- `scripts/check/gate-706-std-wit-full-compliance.py`
