# canonical stringification surface を `to_string(x)` に統一する

**Status**: done
**Created**: 2026-03-29
**Updated**: 2026-04-07
**ID**: 171
**Depends on**: none
**Track**: language-design
**Blocks v1 exit**: no
**ADR candidate**: yes


---

## Decomposition note — 2026-04-03

この issue を 3 層に分解した。

| Layer | Issue | Scope |
|-------|-------|-------|
| ADR / design decision | #483 | `to_string(x)` ポリシーを ADR に記録 |
| implementation | #484 | compiler/stdlib で to_string() を実装 |
| docs + fixtures | **#171 (this issue)** | docs/quickstart 更新 + fixture coverage |

**Close order**: #483 → #484 → #171
**#171 はこの順序の最後**: 実装 (#484) が完了するまで docs を更新してはならない。

---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/171-canonical-to-string-surface.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

Arukellt には `i32_to_string` などの primitive helper、`f"..."`、`Display`/method syntax が混在している。
LLM と user-facing docs の主導線を安定させるため、canonical stringification surface を `to_string(x)` に統一する。

## 受け入れ条件

1. ADR で `to_string(x)` を canonical、`.to_string()` を secondary sugar として記録する
2. compiler / emitter / manifest / LSP が `to_string` を public surface として一貫して扱う
3. docs / quickstart / cookbook の主要サンプルが `to_string(x)` を第一表記にする
4. builtin scalar / char / String と Display-based struct の fixture coverage がある
5. issue index / dependency graph が再生成されている

## 実装タスク

1. `docs/adr/` に stringification policy の ADR を追加する
2. `std/manifest.toml` と stdlib metadata を見直し、`to_string` を canonical surface として扱う
3. emitter の generic `to_string` dispatch の穴を埋める
4. quickstart / syntax / cookbook / migration docs の代表例を更新する
5. `tests/fixtures/stdlib_io/to_string.ark` などの coverage を追加する

## 参照

- `docs/adr/ADR-004-trait-strategy.md`
- `docs/adr/ADR-012-stringification-surface.md`
- `std/manifest.toml`
- `crates/ark-parser/src/parser/expr.rs`

---

## Close evidence — 2026-04-07

- `grep "to_string" docs/language/guide.md` → multiple hits including canonical `#### Stringification — \`to_string(x)\`` section
- `<!-- fixture: stdlib_core/to_string_i32.ark -->` link in docs/language/guide.md
- Canonical `to_string(x)` documented for i32/f64/bool/i64/String in guide.md and stdlib/reference.md
- New fixture: `tests/fixtures/stdlib_core/to_string_i64.ark` (+ `.expected`) added and registered in manifest.txt
- `bash scripts/run/verify-harness.sh --quick`: 19/19 PASS
- `python3 scripts/check/check-docs-consistency.py`: PASS
