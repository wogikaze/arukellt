---
Status: done
Created: 2026-09-19
Updated: 2026-09-27
ID: 877
Parent: 852
Track: language-v2
Depends on: "857, 858, 860, 875, 876"
Related: "ADR-058, RFC-011, #852"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 1
Source: "RFC-011 44, 45"
---

# 877 — normative examples と操作的意味論を実行可能にする

## Scope

RFC-011: **44, 45**。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Acceptance

- [x] normative example を compile/run/check fixture に接続する
- [x] skip に reason/owner/exit condition を必須化する
- [x] observable semantics を集約する
- [x] zero-from-scratch implementation に必要な意味論を散在させない
- [x] CI で normative docs consistency を検証する

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## False-done guard

- [x] Acceptance の一部だけで done にしない
- [x] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [x] 親 #852 の担当項目を evidence 付きで更新する

## Progress receipt — 2026-09-27

### normative example fixture connection

- New fixture domain `tests/fixtures/spec_examples/` (10 entries):
  `run:` array_comprehension, memory_model_move, numeric_literal_suffixes,
  syntax_let_mut_fn, type_annotations, where_clause_magnitude;
  `check-only:` clause_guards, multi_clause_functions, panic_unreachable;
  `compile-error:` multi_clause_pub (pub clause groups rejected E0101).
All 6 run fixtures verified component-compile + wasmtime output match;
  panic_unreachable check-ok; multi_clause_pub rejects as designed.
- Existing-fixture links: spec.md Option::Some clause + let-generalization
  to selfhost/typescheme_poly_let.ark; tuple/array to variables/tuple.ark,
  arrays/array_literal.ark, arrays/array_repeat.ark; ? example to
  question_mark/basic_propagate.ark.
- Remaining doc markers in normative docs are all kind=\"grammar-fragment\"
  (33 in spec.md + 6 syntax + 1 eh + 1 ts): metavar/ellipsis notation
  sketches — permanent notation, not deferred work.

### skip discipline (reason/owner/exit-condition)

- check-doc-examples.py extended: skip-doc-check now requires non-empty
  reason + owner + (expires=\"YYYY-MM-DD\" or exit=\"...\");
  kind in {grammar-fragment, invalid-example, notation} is permanent
  notation requiring reason only; fixture: must resolve to a real file
  registered in tests/fixtures/manifest.txt; expired skips FAIL inside
  normative-class docs (per language-doc-classifications.toml), WARN else.
- Full docs/ run: 10 pass, 0 fail, 275 skip, 0 marker-fail, 0 marker-warn.
- docs/language/ run: 4 pass, 0 fail, 103 skip, 0 marker-fail.

### observable semantics consolidation (RFC-011 item 45)

- docs/data/observable-semantics.toml: 29 rules across all 7 required
  domains (alias, copy, evaluation, numeric, unicode, closure, trait);
  every rule anchors to manifest-registered fixtures + ACCEPTED ADRs;
  28 verified, 1 provisional (closure_escape_boundaries -> #891).
- scripts/check/check-observable-semantics.py: schema, domain coverage
  (>=1 verified rule per required domain), fixture existence + manifest
  registration, ACCEPTED-ADR anchors, issue required for non-verified.
- docs/language/observable-semantics.md: generated normative doc
  (registered in .generated-files + classifications toml).

### CI wiring

- check-observable-semantics.py added to verify quick checks and the
  docs check group (via docs_domain.checks.run_observable_semantics).
- Extended check-doc-examples.py marker validation runs wherever the
  existing gate already runs (verify quick extended, docs check).
- verification-commands.toml: observable_semantics_registry +
  doc_example_check entries added.

### Surfaced defects (split, in scope of #852)

- **#894** — wasm32-gc emit produces invalid core Wasm when one file has
  multiple multi-clause function groups (expected (ref null $type),
  found i64 / inverse). Bisect: single groups fine, pairs fail; wasm32
  linear fine. clause_guards + multi_clause_functions stay check-only:
  (typecheck-level) until #894 lands.
- Stale doc syntax fixed in place: spec.md fn(x) { x } -> |x| x;
  memory-model.md Vec_new_i32() -> Vec::new<i32>(); type-system.md
  [i32; 3] array annotation dropped (unparseable) -> literal form.

## Close receipt — 2026-09-27

Issue-close-review APPROVE (self-review vs false-done criteria):

- implementation commit: `0f8e97eea`
- normative example -> fixture: 6 run fixtures RUN-OK + 6/6 cross-backend
  parity (.build/parity/report-877.json); 3 check-only + 1 compile-error
  registered; 10 spec.md blocks + 5 other normative blocks fixture-linked;
  41 remaining markers all kind="grammar-fragment" (permanent notation).
- skip discipline: full docs/ run `10 pass, 0 fail, 275 skip,
  0 marker-fail, 0 marker-warn`; reason/owner/exit enforced, expired
  skips FAIL inside normative-class docs.
- observable semantics: docs/data/observable-semantics.toml — 29 rules,
  7/7 RFC-011 domains verified; generated normative doc + independent
  checker `check-observable-semantics.py` (PASS).
- CI wiring: verify quick gate entry + docs check group
  (5/5 PASS) + verification-commands.toml entries.
- verification run: docs check 5/5, verify lane PASS (0 fails),
  fmt 10/10 new fixtures.
- tracked remainders: #894 (wasm32-gc emit for multi-clause groups —
  removal condition = upgrade clause_guards + multi_clause_functions to
  run: when emit validates), #891 (escape env cells — provisional rule
  tracking), grammar-fragment markers carry explicit kind+reason.
- parent #852 checklist updated with evidence.
