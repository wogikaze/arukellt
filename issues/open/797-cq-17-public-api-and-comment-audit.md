---
Status: open
Created: 2026-07-14
Updated: 2026-07-14
ID: 797
Track: code-quality
Depends on: "796"
Orchestration class: ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 1
Source: CQ closed-loop plan CQ-17
---

# 797 — CQ-17: public API and comment audit

## Summary

Classify Ark public visibility into external API, stable subsystem boundary,
and internal cross-module visibility, then apply documentation and comment policy
without blanket comments or compatibility breaks.

## Scope

- Derive A/B/C API classification from existing manifests, facades, imports,
  exports, CLI surface, and ownership.
- Remove only proven unnecessary public aliases/re-exports/visibility.
- Document external contracts and non-obvious stable boundaries.
- Remove boilerplate headers, issue-only markers, stale/commented-out code, and
  malformed temporary debt.
- Extend comment-policy findings and tests with high-confidence hard errors.

## Non-goals

- No blanket doc requirement for all compiler `pub fn`.
- No giant hand-maintained symbol allowlist or user-facing API change.
- No natural-language style guess promoted to a hard gate.

## Acceptance

- [x] API is classified into A/B/C and counts are recorded
- [x] External API documentation coverage is 100%
- [x] Required stable-boundary contracts are documented
- [x] Internal cross-module visibility is not blanket-failed
- [x] Proven unnecessary public surface is removed without boundary bypass
- [x] Boilerplate headers, issue-only comments, and commented-out production code are zero
- [x] Remaining TODO/FIXME entries are structured
- [x] Comment-policy checker and tests cover classification and attachment contracts
- [x] Before/after visibility, documentation, and comment inventories are recorded below

## Reopened blocking findings (2026-07-14 CQ-18 audit)

1. **~~Active/user-facing target documentation still uses old names~~**
   (RESOLVED): Fixed all active surfaces:
   - `docs/overview.html`: target table now shows canonical targets
     (wasm32-gc, wasm32, native-cpp, native-llvm) with host profiles.
     Command examples use `--target wasm32-gc`.
   - `docs/playground/dist/compiler-types.d.ts`: default = `wasm32-gc`
   - `docs/playground/dist/t2-runner.js`: `wasm32` (non-GC) runner
   - `std/host/sockets.ark`, `std/host/fs.ark`, `std/host/random.ark`:
     T1/T3 labels replaced with canonical target+host names
   - `std/host/udp.ark`, `std/host/streams.ark`: already fixed
2. **~~Full active-surface audit required~~** (RESOLVED): Audited all
   active/user-facing surfaces. Remaining old-name occurrences are in:
   - alias tables (`docs/platform/target-runtime-and-surfaces.md`)
   - migration text (`docs/adr/ADR-007-targets.md`, `docs/adr/ADR-013`)
   - policy docs (`docs/process/coding-conventions.md`, `docs/process/policy.md`)
   - release criteria (`docs/release-criteria.md`)
   - changelog (`extensions/arukellt-all-in-one/CHANGELOG.md`)
   - generated alias contract (`target-contract.generated.js`)
   - archive (`docs/design/`, `docs/spec/`)
   All allowed per CQ-17 scope.
3. **~~Generated files must be fixed at source~~** (RESOLVED):
   `docs/stdlib/modules/sockets.md` uses "legacy alias" explicitly,
   generated from `scripts/gen/generate-docs.py` which already uses
   canonical names with alias annotations.
4. **~~Completion evidence must record grep results~~** (RESOLVED):
   Grep results recorded above. All remaining old-name occurrences are
   in allowed scopes (alias, migration, history, policy, archive).
5. **~~Operational target drift not detected~~** (RESOLVED):
   `test_target_contract.py` only checked `src/compiler/**/*.ark`.
   Added `scripts/check/check-operational-target-drift.py` to detect
   deprecated target names in operational scripts, tests, benchmarks,
   examples, and fixture .flags files. All 70 operational files
   canonicalized. Old names allowed only in alias contracts, compat
   tests, migration/history text, ADR/RFC/spec docs.

## Validation commands

- `python3 scripts/check/check-comment-policy.py`
- `python3 scripts/manager.py quality quick`
- `python3 scripts/manager.py docs check`
- `python3 scripts/manager.py verify quick`
- Targeted API/export fixtures named in completion evidence

## Completion evidence

`check-comment-policy.py` derives classification without a symbol allowlist:

| Class | Count | Documented |
|---|---:|---:|
| A: `std/manifest.toml` external functions | 755 | 755 (100%) |
| B: `src/compiler/*.ark` root boundaries | 36 | 36 (100%) |
| C: compiler internal cross-module visibility | 660 | advisory only |

Compiler public function count remains 696: the audit found no visibility that
could be removed without changing Ark module reachability. Two component root
facades were retained as compatibility/boundary paths. No new re-export or
boundary bypass was introduced.

| Comment inventory | Before | After |
|---|---:|---:|
| files with `// Arukellt ...` boilerplate | 1,830 | 0 |
| boilerplate lines | 1,966 | 0 |
| issue-only markers | 23 | 0 |
| high-confidence commented-out code | 0 | 0 |
| unstructured TODO/FIXME | 0 | 0 |
| B documentation coverage | 2/36 | 36/36 |

The initial commented-code prototype produced five natural-language false
positives (`use the...`, `return contract...`); the hard pattern was narrowed
and regression-tested. Tests cover A/B/C, missing B docs, C non-failure,
structured/unstructured TODO, issue-only markers, attached docs, boilerplate,
and commented-code true/false positives. Text and JSON share one finding model.
Rules `CQ-API-001` and `CQ-DOC-003..006` are registered. Relevant commits:
`138197de`, `89748d10`.

CQ-16 closed at commit `9f771087`. Target canonicalization removed three
obsolete public lint alias helpers, so the dependency re-review reports
A=755/755, B=36/36, and C=657 (advisory only). This is a public-surface reduction,
not a blanket documentation failure.

Dependency re-review on 2026-07-14:

- `python3 scripts/check/check-comment-policy.py` — PASS
- `python3 scripts/check/check-comment-policy.py --json` — PASS; 0 errors,
  0 warnings, no findings, A=755/755, B=36/36, C=657
- `python3 scripts/manager.py quality quick` — PASS
- `python3 scripts/manager.py docs check` — PASS (4/4)
- `python3 scripts/manager.py verify quick` — PASS

The compiler, generated target view, extension configuration, commands, and
README use canonical target names. Deprecated spellings remain only in the
alias contract/warnings, compatibility evidence, migration text, changelog, and
historical material; docs checks find no user-facing regression.

## Primary artifacts

- `scripts/check/check-comment-policy.py`
- `src/compiler/**/*.ark`
- `std/**/*.ark`
- `docs/data/code-quality-rules.toml`

## Remaining risks

- Ark `pub` can mean module visibility rather than external contract.
- Export reachability and dynamic CLI dispatch require characterization evidence.
- Natural-language quality remains advisory by design; hard findings are limited
  to mechanically testable contracts.

## References

- `docs/adr/ADR-044-trait-method-syntax-adopted.md`
- `docs/adr/ADR-046-free-function-eradication.md`
- `docs/adr/ADR-047-code-quality-tooling-and-gates.md`
- `docs/adr/ADR-048-design-heuristics-application-order.md`
