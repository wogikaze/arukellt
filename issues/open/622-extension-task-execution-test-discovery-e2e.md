---
Status: open
Created: 2026-05-14
Updated: 2026-05-14
ID: 622
Track: vscode-ide
Depends on: 254
Orchestration class: implementation-ready
Orchestration upstream: None
---

# Extension task execution and test discovery E2E

## Summary

#254 closed the extension E2E audit slice, but it intentionally deferred two
larger assertions: executing provided VS Code tasks and asserting discovered
test items, not only controller presence. Existing #273/#274 coverage verifies
registration/fetching and restart health, but does not prove `executeTask`
exit-code behavior or rich test item discovery.

## Why this matters

- Task provider tests can pass while executing a task launches the wrong command
  or reports the wrong exit status.
- Test controller presence can pass while `.ark` test functions are not
  discoverable as VS Code test items.
- These are user-facing IDE workflows that should fail before release.

## Acceptance

- [ ] A VS Code extension-host E2E test executes an Arukellt task via
  `vscode.tasks.executeTask`.
- [ ] Task execution test asserts command arguments and success/failure outcome.
- [ ] Test controller E2E opens a fixture `.ark` file and asserts discovered test
  items, not only controller registration.
- [ ] The extension test suite runs these checks in the existing CI/full local
  gate or documents a gated prerequisite.
- [ ] `npm test` passes in `extensions/arukellt-all-in-one`.

## Primary paths

- `extensions/arukellt-all-in-one/src/test/extension.test.js`
- `extensions/arukellt-all-in-one/src/extension.js`
- `extensions/arukellt-all-in-one/package.json`
- `scripts/gate/ci-full-local.sh`

## Close gate

All acceptance items checked with repo-internal evidence; #254 does not regain
unchecked future bullets.
