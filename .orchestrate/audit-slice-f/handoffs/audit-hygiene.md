<!-- orchestrate handoff
task: audit-hygiene
branch: master
-->

## Status
success

## Branch
`master`

## What I did
- Audited hygiene batch (#373–377, #417–427, #465) against repo scripts and manager.py gates.
- Confirmed #373, #417, #421, #427 truly-done (`check-generated-files.sh`, asset naming, links in verify quick).
- Reopened #418/#422 for missing orphan inventory and admission gate scripts.
- Documented #426 doc drift (verify-harness retired; manager.py carries checks) as monitor-only.

## Verification
type-check-only

Hygiene reopen moves only; no product code touched.

## Notes, concerns, deviations, findings, thoughts, feedback
- `check-orphan-inventory.sh` and `check-admission-gate.sh` absent after #537 shell removal without manager.py migration.

## Suggested follow-ups
- Wire orphan inventory into `manager.py verify quick` when reimplementing #418.
