---
Status: done
Created: 2026-07-14
Updated: 2026-09-16
ID: 706
Track: stdlib
Depends on: 606
Priority: 1
Source: 2026-09-15 verification audit of the merged gc-host snapshot
Action: Re-closed after the verification audit; canonical parser and scalar compiler boundary are covered by the close gate (2026-09-16).
---

# 706 — std::wit Full WIT 1.0 Compliance

## Resolution — 2026-09-16

`std::wit` が WIT 1.0 構文の parser、名前変換、型変換を所有し、compiler は
その結果を compiler metadata へ変換する scalar adapter だけを持つ契約で閉じた。

`WitNode` は再帰 enum なので、現行の `wasm32-gc` lowering では compiler module へ
値として返すと core Wasm の型検証を壊す。したがって、`parse_full` は `std::wit`
内の AST API として維持し、compiler import 経路は同じ `parse_wit_document` を
`std::wit::ast::parse_import_text` 内で一度だけ実行して String metadata に封印する。
compiler は `parser::parse_full_import_text` を一度だけ呼び、source text を再 parse
しない。この境界は recursive AST の ABI を公開せず、parser の二重実装も作らない。

## 現在の受け入れ状況

- [x] Full WIT 1.0 syntax parser が `std::wit::ast` に存在し、`parse_full` から利用できる。
- [x] Shared kebab/snake/Pascal helpers と AST→WIT type surface が `std::wit` に存在する。
- [x] Compiler の重複した `wit_parse_types.ark` model は削除されている。
- [x] Compiler WIT import が canonical `parse_full` parser の scalar adapter を一回だけ呼び、resolver/codegen 用 metadata へ変換する。
- [x] Compiler-local syntax parser を削除し、compiler 側を metadata model と boundary adapter に限定する。
- [x] Full compliance close gate と WIT import fixture parity が同時に通る。

## 再開時の完了条件

1. `std::wit::parser::parse_full` が canonical `std::wit::ast` parser entry point として存在し、
   import adapter (`parse_full_import_text`) が同じ parser implementation を一回だけ実行する。
2. `WitNode` の package/interface/function/record/enum/variant 情報を `std::wit` 内で String
   metadata へ封印し、compiler がその metadata を resolver/codegen 用 model へ変換する。
   compiler は source text をもう一度 parser に渡さない。
3. named type、component import の diagnostics、WIT binding の既存 fixture を維持する。
4. bootstrap compiler で WIT import fixture を compile/validate/execute する。
5. `python3 scripts/check/gate-706-std-wit-full-compliance.py` と
   `python3 scripts/check/gate-component-wit-productization.py` が PASS する。

## Verification evidence

- `python3 scripts/check/gate-706-std-wit-full-compliance.py` — PASS
- `python3 scripts/check/gate-652-wit-import-parser.py` — PASS
- `python3 scripts/check/gate-653-wit-import-resolver-mir.py` — PASS
- `python3 scripts/check/gate-654-wit-import-component-emit.py` — PASS
- `python3 scripts/check/gate-663-ark-toml-wit-package.py` — PASS
- `python3 scripts/check/gate-664-wit-import-record-enum-bindings.py` — PASS
- `python3 scripts/check/gate-665-wit-import-compose-roundtrip-e2e.py` — PASS
- `python3 scripts/check/check-component-wit-parse.py` — PASS
- `tests/fixtures/stdlib_wit/wit_ast_parse.ark` — core Wasm and component validate;
  Wasmtime execution prints `OK`.

## References

- `std/wit/ast.ark`
- `std/wit/parser.ark`
- `src/compiler/component/wit_parse_import.ark`
- `src/compiler/component/wit_parse_text.ark`
- `scripts/check/gate-706-std-wit-full-compliance.py`
