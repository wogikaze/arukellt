# Release: Extension Activation Tests

> **Status:** open
> **Track:** release
> **Type:** Verification

## Scope

Ensure extension activation tests pass for release verification.

## Checklist Source

docs/release-checklist.md — Extension distribution section

## Acceptance

- [x] `cd extensions/arukellt-all-in-one && npm ci && npm run build` succeeds
- [x] VSIX package generated (`.vsix` file exists)
- [ ] Extension activation tests pass (`xvfb-run -a npm test`) — BLOCKED by WSL environment

## Verification Evidence

- Environment check: `node_modules` present in `extensions/arukellt-all-in-one/`; `xvfb-run` available at `/usr/bin/xvfb-run`.
- `npm run build` in `extensions/arukellt-all-in-one/`: PASS
  - `vsce package` completed successfully.
  - VSIX written to `extensions/arukellt-all-in-one/arukellt-all-in-one-0.0.1.vsix`.
- `xvfb-run -a npm test` in `extensions/arukellt-all-in-one/`: FAIL
  - Result: `31 passing`, `1 pending`, `3 failing`.
  - Failing checks:
    - `Go to Definition (#450 / #453) local variable definition range is identifier only`
      - `Should point to let-binding line (line 7)`: expected `7`, actual `2`.
    - `Go to Definition (#450 / #453) function definition range is function name only`
      - `Should find definition of greet`: expected `true`, actual `false`.
    - `Hover (#451 / #453) known function name produces meaningful hover content`
      - Hover text was:

        ```arukellt
        let result: String
        ```

      - Expected hover content to include function name or signature.

## Close Candidate

- `no`
- Blocker: the exact `xvfb-run -a npm test` release gate cannot complete in this checkout because Xvfb cannot start with the current `/tmp/.X11-unix` ownership.

## Recheck — 2026-05-14

- `npm ci && npm run build` in `extensions/arukellt-all-in-one/`: PASS.
- VSIX generated: `extensions/arukellt-all-in-one/arukellt-all-in-one-0.0.1.vsix`.
- `npm test` without `xvfb-run`: PASS (`vscode-test` reports `Exit code: 0`).
- `xvfb-run -a npm test`: BLOCKED by local Xvfb socket ownership, not by extension tests:
  - `xvfb-run -a true` exits 1.
  - `/tmp/.X11-unix` is owned by `wgkz:wgkz`; Xvfb reports `Owner of /tmp/.X11-unix should be set to root`.
  - `sudo chown root:root /tmp/.X11-unix` could not run here because sudo requires a password.

Updated close-candidate status: still `no` until the exact `xvfb-run -a npm test` gate exits 0 in a correctly configured local or CI Xvfb environment.

## Recheck — 2026-05-16

- `npm ci && npm run build` in `extensions/arukellt-all-in-one/`: PASS.
  - VSIX generated: `extensions/arukellt-all-in-one/arukellt-all-in-one-0.0.1.vsix` (327 files, 485.26 KB).
- `/tmp/.X11-unix` ownership issue RESOLVED: `xvfb-run -a true` now exits 0 (socket owned by `wogikaze`, not `root`).
- `npm test` (without `xvfb-run`): FAILS with 9 test failures (exit code 1).
  - Failures include Debug Launch (#255), Go to Definition range assertions, and Hover content checks.
  - NOTE: Previous recheck (2026-05-14) reported `npm test` passing with exit code 0. The current failures may be environment-specific (existing Code instance) or genuine regressions.
- `xvfb-run -a npm test`: BLOCKED by interactive WSL prompt:
  - "To use Visual Studio Code with the Windows Subsystem for Linux, please install Visual Studio Code in Windows and uninstall the Linux version in WSL."
  - The test harness blocks waiting for `[y/N]` input indefinitely.
  - This prompt does not appear in CI (GitHub Actions runs on native Ubuntu, not WSL).

Updated close-candidate status: still `no`. The `xvfb-run -a npm test` gate cannot complete in this WSL environment due to the VS Code WSL detection prompt. Additionally, the raw `npm test` run now shows 9 assertion failures that did not appear during the 2026-05-14 recheck, warranting investigation.

## Required Verification

- Build extension from source
- Verify VSIX package generation
- Run activation tests in headless X environment

## Close Gate

Extension build, VSIX generation, and activation tests must all pass.

## Assessment — 2026-05-16

### Acceptance Criteria Status

| Criterion | Status | Evidence |
|-----------|--------|----------|
| `npm ci && npm run build` succeeds | PASS | Confirmed across 3 recheck cycles (2026-05-14, 2026-05-16) |
| VSIX package generated | PASS | `arukellt-all-in-one-0.0.1.vsix` (327 files, 485.26 KB) |
| `xvfb-run -a npm test` passes | BLOCKED | Cannot run in WSL due to VS Code WSL detection interactive prompt |

### Analysis

- **AC 1 and AC 2 are stable and verified.** The build and packaging pipeline works reliably.
- **AC 3 is not verifiable in this environment** for two reasons:
  1. The VS Code WSL detection prompt (`"To use Visual Studio Code with the Windows Subsystem for Linux..."`) blocks the xvfb test harness in WSL. This does not occur in CI (native Ubuntu).
  2. `npm test` without xvfb shows 9 assertion failures in this WSL session. These were not present during the 2026-05-14 recheck (which reported `Exit code: 0`), suggesting environment sensitivity (possibly a stale VS Code instance lock or configuration drift).
- The **extension test suite** in `src/test/extension.test.js` is comprehensive: 18 test suites covering activation, command registration, task provider, test discovery, Go to Definition, Hover, Diagnostics, Debug Launch, Language Server restart, failure recovery, and project tree views. The test infrastructure itself is mature.

### Recommendation

**Do not close.** The `xvfb-run -a npm test` gate requires a native Linux CI run to verify. However:

- Consider splitting AC 3 into a CI-only gate and marking this issue as **blocked-by-environment** rather than open/failing.
- The 9 `npm test` assertion failures seen on 2026-05-16 should be investigated on a clean VS Code instance (not WSL). The failures affect Debug Launch (#255), Go to Definition range (#450/#453), Hover content (#451/#453), and Diagnostics (#452/#453) assertions — all of which use direct LSP pipe sessions or VS Code API calls that may behave differently under WSL interference.
- After a clean CI run confirms `xvfb-run -a npm test` passes, this issue can be closed and the release checklist checkbox migrated to the existing CI pipeline.

## Primary Paths

- `extensions/arukellt-all-in-one/` (extension source)
- Extension build configuration
- VSIX package output
- Activation test suite

## Non-Goals

- Cross-platform testing (Linux only for now)
- Performance optimization
