---
Status: open
Created: 2026-06-17
Updated: 2026-06-17
ID: 683
Track: docs-audit
Depends on: 679, 682
Orchestration class: audit-ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 2
Source: Quickstart executable example audit framework 2026-06-17
Child tracks: 671, 461
---

# 683 — User-facing executable example audit (Quickstart / skip-doc-check)

## Summary

`docs/quickstart.md` は `--wit host.wit` 例や fs/env 例を掲げるが、
`<!-- skip-doc-check -->` が複数節に残り、doc example が verify 外である。
`docs/stdlib/recipe-manifest.toml` と fixture リンクの完全性も監査対象。

（注: #034 以降、`--wit` callable import は `tests/fixtures/wit_import/` で証明済み。
本 issue は **Quickstart 読者がたどれる success path** の有無を問う。）

## Audit checklist (section 1 + 10)

| チェック | 現状 (2026-06-17) | 起票/追跡 |
|----------|-------------------|-----------|
| Quickstart 例が manifest fixture に紐づく | Hello は `hello_world` 相当；Component/`--wit` は fixture リンクなし | 本 issue |
| `--wit` に end-to-end success fixture | `wit_import/main.ark` あり；Quickstart 未参照 | 本 issue |
| `skip-doc-check` が user-facing に残存 | quickstart 2箇所 + language/stdlib 多数 | 本 issue |
| docs example が manifest と型一致 | fs 節 `read_to_string` vs 実 API 名要確認 | 本 issue |
| examples/README が Quickstart から到達可能 | quickstart L29 リンクあり | OK |
| cookbook recipe-manifest 完全性 | `check_recipe_fixture_links` あり | 要失敗行監査 |

## Acceptance

- [ ] Quickstart 全 code block が `tests/fixtures/` または `examples/` の path と対応
      （`docs/quickstart.md` 先頭に fixture index 表）
- [ ] Component + `--wit` 節が `tests/fixtures/wit_import/main.ark` をコピペ可能な形で掲載
- [ ] Quickstart から `skip-doc-check` を除去（#461 残タスクを子 issue 化しても可）
- [ ] `scripts/check/check-doc-examples.py` が quickstart を **常時 enforce**（skip 禁止）
- [ ] Gate `scripts/check/gate-683-quickstart-executable-audit.py`
- [ ] `python3 scripts/manager.py verify quick` exits 0

## References

- `docs/quickstart.md`
- `docs/stdlib/recipe-manifest.toml`
- `scripts/check/check-doc-examples.py`
- `issues/done/461-stdlib-io-buffer-api.md` (skip-doc-check 起因)
