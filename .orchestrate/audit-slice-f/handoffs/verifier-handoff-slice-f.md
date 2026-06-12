<!-- orchestrate handoff
task: verify-slice-f (independent verifier)
branch: master
-->

## Verification
verifier-blocked

## Target
`audit-slice-f` on branch `master`

## Branch
`master`

## Execution
- `python3 scripts/manager.py verify quick` → exit 1; **143/149 passed, 6 failed** (log: `.orchestrate/audit-slice-f/handoffs/verify-quick-2026-06-12.log`)
- `ls scripts/check/check-orphan-inventory.sh scripts/check/check-admission-gate.sh` → both paths absent (ENOENT)
- `rg '418-hygiene|422-hygiene' issues/open/index.md` → both issues listed in open index with `Status: open` entries
- `git show 9459feec --stat` → Slice F commit touches only `issues/**`, audit report, orchestration artifacts (no `scripts/manager.py` gate changes)
- Read `docs/process/false-done-audit-2026-06-12.md` § Slice F → documents 57 reviewed, 2 reopened (#418/#422), 6 audit-resolved

## Findings
Per acceptance criterion:
- [x] Release/hygiene false-done handled: #418/#422 moved to `issues/open/` with audit reopen sections; missing-script evidence confirmed; index/dependency-graph regenerated (met)
- [x] Audit report updated: `docs/process/false-done-audit-2026-06-12.md` contains Slice F wave (§331+) (met)
- [ ] verify quick pass: 143/149 pass, 6 failures — **not met** (full suite does not pass)

Other findings (severity-ordered):
- (high) verify quick exit 1 — six pre-existing failures unchanged, none attributable to Slice F diff: wasmtime missing (#568/#569/#571), `arukellt` binary missing (doc examples), generated docs drift, #487 STATUS_MISMATCH in false-done hygiene gate
- (med) Reopen rationale validated: cited hygiene scripts never migrated to manager.py after #537 shell removal
- (low) #544 bench directory layout vs stated goals flagged monitor-only in audit report; not reopened

## Notes & suggestions
- Slice F audit work is substantively correct; blocker is environmental/pre-existing verify harness gaps, not slice regressions.
- To achieve full verify quick pass in CI-like env: install wasmtime, build `arukellt`, run `python3 scripts/gen/generate-docs.py`, resolve #487 status mismatch.
- Follow-up implementation track: #418 (orphan inventory gate) unblocks #422.
