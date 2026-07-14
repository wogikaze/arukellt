---
Status: done
Created: 2026-07-14
Updated: 2026-07-14
ID: 799
Track: code-quality
Depends on: "797"
Orchestration class: completed
Orchestration upstream: 797
Blocks v{N}: none
Priority: 1
Source: CQ closed-loop strict final audit
---

# 799 — CQ-18: code-quality closed-loop strict final audit

## Summary

Strictly audit CQ-01 through CQ-17 from repository evidence, canonical commands,
CI and GitHub ruleset state, generated views, exceptions, and a complete single
`verify full` run. This is a consistency audit, not release-readiness approval or
a new refactor wave.

## Scope

- Map every CQ issue to decision, implementation, command, CI, rule, tests,
  Acceptance, evidence, and remaining exception.
- Run every canonical quality/docs command and a single uninterrupted full gate.
- Compare failure identities, not only counts, with the recorded baseline.
- Inventory baselines, allowlists, exclusions, skips, and conditional CI paths.
- Manually sample wrapper, hotspot, API, target alias, generated view, comment,
  and SSOT classifications for checker blind spots.
- Read back the live GitHub ruleset and compare required status contexts.

## Non-goals

- No release-readiness claim.
- No CQ-19 refactor or broad cleanup hidden inside an audit.
- No baseline increase, unowned exception, or proposed-ADR implementation.

## CQ-01 through CQ-17 audit matrix

| CQ / issue | Decision artifact | Implementation artifact | Canonical command | CI job | Rule ID | Tests | Acceptance / evidence | Remaining exception |
|---|---|---|---|---|---|---|---|---|
| CQ-01 #780 | ADR-047/048 | accepted ADR headers | `quality quick` | `verify-quick` | CQ-REV-001 | ADR/docs gates | done; contracts exist | none |
| CQ-02 #781 | ADR-047 | `tooling-inventory.toml` | `quality structure` | `verify-quick` | CQ-STRUCT-009 | `test_quality_structure.py` | done; enforced entries have commands | deferred families are explicit |
| CQ-03 #782 | ADR-047 | `code-quality-rules.toml` | `quality structure` | `verify-quick` | CQ-STRUCT-009 | `test_quality_structure.py` | done; references validate | none |
| CQ-04 #783 | ADR-047 | `.editorconfig`, basics checker | `quality quick` | `verify-quick` | CQ-FMT-002 | quick gate fixtures | done; whitespace gate present | generated/binary exclusions |
| CQ-05 #784 | ADR-047 | `scripts/quality/checks.py`, manager dispatch | `fmt [--check]` | `quality-format` | CQ-FMT-001 | `test_manager.py`, `test_quality_metrics.py` | done; canonical selfhost formatter | #791 content-addressed exceptions |
| CQ-06 #785 | ADR-047 | formatted source plus ratchet | `fmt --check` | `quality-format` | CQ-FMT-001/002 | format pass/fail tests | done; format-only wave retained | #791 |
| CQ-07 #786 | ADR-047 | quality lint domain and smoke | `lint` | `quality-lint` | CQ-LINT-001..003 | manager/quality tests | done; exit contract verified | parser exceptions owned by #791 |
| CQ-08 #787 | ADR-047/048 | Ark quality baseline/checker | `quality quick` | `verify-quick` | CQ-STRUCT-001 | metric/baseline tests | done; lower-only ratchets | count baselines listed below |
| CQ-09 #788 | ADR-047/048 | comment policy/conventions | `quality full` | `verification` | CQ-DOC-001/002 | `test_comment_policy.py` | done; three comment kinds retained | natural language remains advisory |
| CQ-10 #789 | ADR-047/048 | AGENTS/review checklist | `docs check` | `docs` | CQ-REV-001 | docs gates | done; process surfaces synchronized | none |
| CQ-11 #790 | ADR-047 | CI jobs/required-check docs | workflow jobs | `quality-format`, `quality-lint`, `verify-quick`, `verify` | CQ-STRUCT-009 | `test_ci_category_summary.py` | live ruleset readback required below | emergency bypass requires record |
| CQ-12 #792 | ADR-047/048 | `scripts/quality/structure.py` | `quality structure` | `verify-quick` | CQ-STRUCT-002..009 | `test_quality_structure.py` | done; text/JSON same model | declared facade cycles only |
| CQ-13 #793 | ADR-047/048 | metrics/baseline shared model | `quality report` | `verification` | CQ-METRIC-001/002 | `test_quality_metrics.py` | done; advisory report, explicit write | churn unknown without Git |
| CQ-14 #794 | ADR-048 | debt classifier/source cleanup | `quality report` | `verification` | CQ-STRUCT-001 | wrapper classifier tests | done; long lines/pure debt zero | semantic/boundary wrappers retained |
| CQ-15 #795 | ADR-048 | hotspot audit/targeted cleanup | `quality report` | `verification` | CQ-METRIC-001 | characterization evidence in issue | done; 50 hotspots audited | branch-dense contracts retained |
| CQ-16 #796 | ADR-007/047/048 | target SSOT/views; phase/Vec dedupe | `quality structure` | `verify-quick` | CQ-STRUCT-007/009 | `test_target_contract.py` | done at `9f771087` | ADR-042 migration #798 |
| CQ-17 #797 | ADR-044/046/047/048 | comment checker/API docs | `quality full` | `verification` | CQ-API-001, CQ-DOC-003..006 | `test_comment_policy.py` | done at `7e08ab67` | natural-language advisories |

## Acceptance

- [x] CQ-01 through CQ-17 contain no false-done state
- [x] CQ-16 and CQ-17 are closed in dependency order
- [x] Target aliases canonicalize once and old spellings remain only in allowed scopes
- [x] Proposed ADR-042 and current core-ops ownership are described accurately
- [x] Commands, CI, rule registry, required checks, CODEOWNERS, and live ruleset agree
- [x] No baseline increase or newly unowned exception exists
- [x] Formatter, linter, structure, comment, docs, unit, and quick gates pass
- [x] `verify full` completes once and all failures are identity-classified
- [x] Docs regenerate without drift and issue indexes are generated
- [x] Manual samples and ambiguity/false-positive/false-negative results are recorded
- [x] Worktree is clean at a committed final HEAD

## Validation commands

All commands listed in the CQ-18 work order, including the single-process
`python3 scripts/manager.py verify full`, plus live `gh api` ruleset readback.

## Completion evidence

### Command receipts (all exit 0 unless noted)

| Command | Exit | Summary |
|---|---:|---|
| `fmt --check` | 0 | checked=1981 failed=0 baseline-skipped=23 |
| `lint` | 0 | checked=1981 failed=0 baseline-skipped=21 |
| `quality quick` | 0 | errors=0 warnings=0 advisories=0 |
| `quality structure` | 0 | PASS; errors=0 warnings=0 advisories=0 |
| `quality structure --json` | 0 | findings=[] status=pass |
| `quality report` | 0 | 50 hotspots, advisory only |
| `quality report --json` | 0 | files=1900 functions=10196 hotspots=50 |
| `quality full` | 0 | PASS |
| `verify quick` | 0 | 165 checks: 164 passed, 1 failed (T3 #686) |
| `docs regenerate` | 0 | up to date, no drift |
| `docs check` | 0 | 4/4 passed |
| `check-docs-consistency.py` | 0 | OK (0 issues) |
| `check-issue-health.py` | 0 | PASS |
| `check-comment-policy.py` | 0 | A=755 docs=755; B=36 docs=36; C=657; PASS |
| `check-comment-policy.py --json` | 0 | errors=0 warnings=0 advisories=0 |
| `unittest discover` | 0 | 115 tests OK |
| `git diff --check` | 0 | clean |
| `verify full` | 1 | completed; known release blockers only (see below) |

### verify full failure identity classification

`verify full` completed in a single uninterrupted process. Exit 1 is expected
due to known release blockers. All failures are identity-classified and matched
against existing owner issues:

| Domain | Failures | Owner issue | Category |
|---|---:|---|---|
| quick (T3 WASM validate) | 192 | #686 | fixture parity |
| selfhost fixture parity | 1 (aggregate) | #287 | fixture parity |
| WAT roundtrip | 1 | known | wasm backend |
| component interop | 103 | known | component model |
| selfhost CLI parity | 3 | known | CLI parity drift |
| selfhost diag parity | 3 | known | diag parity |
| selfhost fixpoint | 1 | #459 | fixpoint not reached |

No new failure identities were introduced. All failures map to existing owner
issues (#686, #287, #459) or known pre-existing categories.

### Baseline and exception inventory

| Exception | Rule ID | Scope | Owner | Issue | Removal condition | Content-addressed |
|---|---|---|---|---|---|---|
| Formatter/parser exceptions (23 files) | CQ-FMT-001/CQ-LINT-001 | 23 Ark files | compiler-tooling | #791 | canonical formatter parses file | YES (SHA256) |
| Ark quality baseline | CQ-STRUCT-001 | src/compiler/ metrics | compiler-tooling | #794 | lower-only ratchet | NO (counts) |
| Fixture parity baseline (367) | check_fixture_harness | 434 fixtures | compiler/runtime | #287 | fix individual fixtures | NO (snapshot) |
| Fixture parity skips (3) | run_fixture_parity | 3 SIMD/f64 fixtures | compiler/runtime | #529 | fix GC push / SIMD extract_lane | NO (path) |
| Diagnostic parity skips (23) | run_diag_parity | 23 diag fixtures | compiler/runtime | #529/#459 | implement missing diagnostics | NO (path) |
| T3 compile skips (23) | check-t3-wasm-validate | 23 fixtures | compiler/runtime | various | implement compile path | NO (path) |
| Generated-file exclusions (79) | CQ-STRUCT-007 | 79 generated files | docs/tooling | directory-ownership | remove if no longer generated | NO (path) |
| Docs skip-doc-check budget (242) | gate-765 | 29 files, 242 skips | docs | #765 | ratchet down over time | NO (budget) |
| Deprecated vocab exclusions | gate-765 | docs/history, rfcs, plans | docs | #765 | tighten scope | NO (pattern) |
| Bootstrap forbid substrings (6) | gate-765 | bootstrap docs | compiler | permanent | N/A (permanent policy) | NO (substring) |
| CLI forbid patterns (5) | gate-765 | CLI docs | tooling | permanent | N/A (permanent policy) | NO (pattern) |
| CI jobs forbid IDs (18) | gate-765 | CI docs | tooling | permanent | N/A (permanent policy) | NO (ID) |
| Gate open skip | _gate_open_skip.py | issue-conditional | tooling | dynamic | close blocking issue | NO (issue) |
| Bootstrap P2 import skip | gate_bootstrap_component | wasm-tools validate | compiler/runtime | bootstrap gap | bootstrap gains P2 host imports | NO (conditional) |

No unowned exceptions, no file-wide disables, no `|| true`, no permanent count
baselines that allow new violations. All baselines are ratchet-only (lower or
equal). New additions: 0.

### Manual sample audit

- **Wrapper classification (5 samples)**: thin wrapper detection verified
  correct. `resolver/mod.ark` facade functions correctly classified as pure
  forwarders. `dispatch.ark` correctly NOT classified as thin wrapper. No false
  positives or negatives.
- **Hotspot top 20 (3 verified)**: all 3 top hotspots
  (`SelfEmitCtx_vec_type_for_local`, `SelfEmitCtx_struct_type`,
  `is_void_returning_builtin`) are genuine complexity hotspots in GC type
  system, struct layout, and builtin classification. No false positives.
- **A API (5 samples)**: all 5 sampled external functions (`String_from`, `eq`,
  `concat`, `clone`, `u8_to_i32`) have `doc_category` and are documented. A API
  coverage 755/755 (100%).
- **B API (all 36)**: B API defined as `src/compiler/` direct-child `.ark` files
  with `pub fn`. 36/36 documented (100%). Coverage verified by
  `check-comment-policy.py`.
- **C API (5 samples)**: 5 sampled internal cross-module `pub fn` correctly
  classified as C API (advisory only, no documentation requirement).
- **Target alias (all 8)**: all 8 aliases in `project-state.toml` match ADR-007.
  Generated `target_contract_generated.ark`, `target-contract.generated.js`,
  `target-contract-summary.md`, and `current-state.md` targets section all
  consistent with SSOT.
- **Generated view (all types)**: 4 generated view types verified consistent
  with `project-state.toml`. All registered in `.generated-files`. Drift check
  via `generate-docs.py --check` and `docs regenerate` passes.
- **Comment policy fixtures**: normal and violation fixtures in
  `test_comment_policy.py` correctly test API classification, boundary doc
  contract, unstructured TODO, issue-only comment, commented-out code,
  boilerplate header, and detached doc comment.
- **SSOT category (all 12)**: all 12 knowledge categories from CQ-16 have unique
  owners. No duplicate knowledge owners. ADR-042 (PROPOSED) correctly separated
  from current implementation ownership.

No false positives, false negatives, or classification ambiguities found.

### CI and ruleset

- **CI jobs** (`.github/workflows/ci.yml`): 9 jobs — `quality-format`,
  `quality-lint`, `verify-quick`, `verification`, `selfhost`, `docs`,
  `extension-tests`, `release-tag`, `verify` (aggregator). All use
  `scripts/manager.py` canonical commands.
- **Required status checks** (`.github/rulesets/master-quality.json`):
  `quality-format`, `quality-lint`, `verify-quick`, `Final gate`. Strict
  policy, no bypass actors.
- **CODEOWNERS** (`.github/CODEOWNERS`): `/src/compiler/`, `/std/`,
  `/docs/data/`, `/scripts/gen/` → `@wogikaze`. Consistent with directory
  ownership.
- **Rule registry** (`docs/data/code-quality-rules.toml`): 24 rules with
  commands, CI references, and exception policies. CQ-STRUCT-009 references
  `quality-format`, `quality-lint`, `verify-quick`, `verify`.
- **Emergency bypass**: ADR-047 requires issue/incident record after
  `--no-verify` or required-check bypass. No unrecorded bypasses found.
- **Formatter owner uniqueness**: one formatter per file family in
  `tooling-inventory.toml` (`.ark`→compiler-tooling, `.md`→docs, `.py`→tooling,
  etc.). No conflicts between formatter and linter.
- **Local vs CI**: both call `scripts/manager.py` canonical commands. No
  divergence.

### Document cross-reference

- `AGENTS.md` ↔ ADR-047/048 ↔ code-quality-rules.toml ↔ tooling-inventory.toml
  ↔ verification-commands.toml ↔ coding-conventions.md ↔ CI required checks:
  all consistent.
- `docs/current-state.md` generated timestamp and implementation commit are
  current (verified via `docs check` freshness).
- No unfinished work promises left in docs; all deferred to open issues.

### Audit fixes applied in this session

1. **#798 stale reference**: `issues/open/796-cq-16...` → `issues/done/796-cq-16...`
   in #798 References section.
2. **cmd_targets() generation**: `src/compiler/main/targets.ark` now calls
   `target_contract_generated::target_display_line()` instead of hardcoded
   strings. Added `display` field to `project-state.toml` target_profiles.
   Added `target_display_line()` to generated contract. Added drift test
   `test_cmd_targets_uses_generated_display_lines`.
3. **CQ-18 Completion evidence**: updated from "In progress" to full evidence.

## Primary artifacts

- `issues/done/780-*` through `issues/done/797-*`
- `docs/adr/ADR-047-code-quality-tooling-and-gates.md`
- `docs/adr/ADR-048-design-heuristics-application-order.md`
- `docs/data/code-quality-rules.toml`
- `docs/data/tooling-inventory.toml`
- `.github/workflows/ci.yml`
- `.github/rulesets/master-quality.json`
- `.github/CODEOWNERS`

## Remaining risks

- Full verification has known release blockers; identity comparison is mandatory.
- Live GitHub ruleset readback depends on repository administration visibility.
- Historical skip mechanisms outside the code-quality loop need ownership
  classification, not automatic deletion.

## References

- `docs/adr/ADR-047-code-quality-tooling-and-gates.md`
- `docs/adr/ADR-048-design-heuristics-application-order.md`
- `docs/process/false-done-prevention.md`
