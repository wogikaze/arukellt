---
Status: done
Created: 2026-03-28
Updated: 2026-06-14
ID: 117
Track: wasm-quality
Depends on: —
Orchestration class: implementation-ready
Blocks v4 exit: False
---

# Component Model: WIT 生成品質の向上と往復検証

## Summary

Selfhost `src/compiler/component/wit_text.ark` / `wit_types.ark` / `wit_names.ark`
generate WIT text for component exports. Quality gate covers `option<T>`, `result<T, E>`,
`tuple<T1, T2>`, and kebab-case naming; golden WIT is parsed by `wasm-tools component wit`.

## 受け入れ条件

- [x] `Option<T>` → WIT `option<T>` に変換 (`wit_types.ark`)
- [x] `Result<T, E>` → WIT `result<T, E>` に変換 (`wit_types.ark`)
- [x] タプル型 → WIT `tuple<T1, T2>` に変換 (`wit_types.ark`)
- [x] 生成 WIT を `wasm-tools component wit` でパースできることを verify に追加
      (`component-wit-parse:component/wit_quality.ark`, `scripts/check/check-component-wit-parse.py`)
- [x] kebab-case 変換の一貫性 (`wit_names.ark` — `wit_export_name` / `wit_type_name` 共有)

## Close evidence (2026-06-14)

- Fixture: `tests/fixtures/component/wit_quality.ark` + `wit_quality.expected.wit`
- Manifest: `component-wit-parse:component/wit_quality.ark`
- Gate: `python3 scripts/check/check-component-wit-parse.py` (wired in `verify quick`)
- `python3 scripts/manager.py verify quick` → 157/157 pass

## 参照

- `docs/spec/spec-WASI-0.2.10/OVERVIEW.md` §WIT形式の読み方
- `src/compiler/component/wit_text.ark`
