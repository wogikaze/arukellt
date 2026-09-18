---
Status: done
Created: 2026-09-19
Updated: 2026-09-19
ID: 853
Parent: 852
Track: language-v2
Depends on: none
Related: "ADR-058, RFC-011, #852"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 1
Source: "RFC-011 1–4"
---

# 853 — Ownership / move / Copy / mutability を確定・実装する

## Scope

RFC-011: **1–4**。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Acceptance

- [x] move-after-use を型検査し positive/negative fixture を追加する
- [x] `let` / `let mut` の再代入と mutation 規則を仕様・実装で一致させる
- [x] mutability と `Copy` を独立に扱う
- [x] user-defined struct/enum の `Copy` は明示 opt-in にする
- [x] tuple / fixed array / struct の move/Copy を一貫させる

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## False-done guard

- [x] Acceptance の一部だけで done にしない
- [x] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [x] 親 #852 の担当項目を evidence 付きで更新する

## Completion evidence (2026-09-19)

- Implementation: `ad0c4f21d` and `7fda6273c`.
- Normative contract: [ADR-059](../../docs/adr/ADR-059-language-v2-ownership-copy-mutability.md), with RFC-011 items 1–4 marked adopted.
- Positive fixture: `tests/fixtures/ownership/copy_and_mutability.ark` and `.expected`.
- Negative fixture: `tests/fixtures/diagnostics/ownership_move_after_use.ark` and `.diag` (four E0212 diagnostics for String, struct, tuple, and array reuse).
- GC primary receipt: `wasm32-gc --emit component` validated and ran with output `1 2 1 7 7 7 3 4 5`.
- Linear-memory fallback receipt: `wasm32` validated and ran with the same output.
- `python3 scripts/manager.py verify fixtures`: `PASS=1692 FAIL=0 COMPILE_VALIDATE=1692 EXECUTED=101 RUNTIME_UNSUPPORTED=2`; the two unsupported contracts are existing native-cpp-owned coverage, not skipped ownership checks.
- `python3 scripts/manager.py verify lane`: PASS; `bash scripts/check/check-diagnostic-codes.sh`: PASS (54/54); `python3 scripts/check/check-adrs.py`: PASS; `python3 scripts/manager.py docs check`: PASS.
