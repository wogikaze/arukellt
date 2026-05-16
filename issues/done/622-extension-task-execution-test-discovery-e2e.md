---
Status: done
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

- [x] A VS Code extension-host E2E test executes an Arukellt task via
  `vscode.tasks.executeTask`.
- [x] Task execution test asserts command arguments and success/failure outcome.
- [x] Test controller E2E opens a fixture `.ark` file and asserts discovered test
  items, not only controller registration.
- [x] The extension test suite runs these checks in the existing CI/full local
  gate or documents a gated prerequisite.
- [x] `npm test` passes in `extensions/arukellt-all-in-one`.

## Primary paths

- `extensions/arukellt-all-in-one/src/test/extension.test.js`
- `extensions/arukellt-all-in-one/src/extension.js`
- `extensions/arukellt-all-in-one/package.json`
- `scripts/gate/ci-full-local.sh`

## Close gate

All acceptance items checked with repo-internal evidence; #254 does not regain
unchecked future bullets.

## Completion evidence — 2026-05-14

- Added extension-host E2E coverage for executing provided Arukellt tasks via
  `vscode.tasks.executeTask`.
- Added command-argument and exit-code assertions using a temporary CLI stub.
- Added fixture-backed test discovery coverage that opens a `.ark` file and
  asserts concrete discovered test item labels.
- Added a small test-only extension export for deterministic discovery
  assertions.
- `.vscode-test.mjs` passes `--no-sandbox` and reuses an already downloaded
  `bin/code` executable when present, avoiding container-only sandbox traps
  while preserving the normal CI download path.
- The existing `extension-tests` CI job runs `xvfb-run -a npm test`; the local
  `npm test` gate now exercises the added checks.
- Verification:
  - `npm test` → PASS
  - `npm run test:marketplace-metadata` → PASS
  - `npm run build` → PASS
  - `node --check src/extension.js` → PASS
  - `node --check src/test/extension.test.js` → PASS
  - `git diff --check` → PASS
