---
Status: done
Updated: 2026-05-17
ID: 549
Track: release
Type: Verification
Depends on: none
---

# Release: Extension Activation Tests

> **Status:** done
> **Track:** release
> **Type:** Verification

## Scope

Ensure extension activation tests pass for release verification.

## Checklist Source

docs/release-checklist.md — Extension distribution section

## Acceptance

- [x] `cd extensions/arukellt-all-in-one && npm ci && npm run build` succeeds
- [x] VSIX package generated (`.vsix` file exists)
- [x] Extension activation tests pass (`xvfb-run -a npm test`) — verified in native Ubuntu CI

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

- `yes`
- Native Ubuntu CI now runs and passes the extension activation gate. This local
  WSL checkout still cannot provide a trustworthy Xvfb socket directory, but
  that is no longer blocking because the release gate is covered by CI evidence.

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

## Recheck — 2026-05-17

- `npm ci` in `extensions/arukellt-all-in-one/`: PASS (392 packages installed).
- `npm run build` in `extensions/arukellt-all-in-one/`: PASS.
  - `vsce package` completed.
  - VSIX written to `extensions/arukellt-all-in-one/arukellt-all-in-one-0.0.1.vsix`
    (327 files, 485.26 KB).
- `npm run test:marketplace-metadata`: PASS (`marketplace metadata OK`).
- Downloaded VS Code script check:
  - `.vscode-test/vscode-linux-x64-1.120.0/bin/code` suppresses the WSL prompt
    when `DONT_PROMPT_WSL_INSTALL` is set.
- `env DONT_PROMPT_WSL_INSTALL=1 npm test`: PASS (`Exit code: 0`).
- `xvfb-run -e /dev/stdout -a true`: FAIL.
  - Xvfb reports `/tmp/.X11-unix` owner must be `root` and mode must be `1777`.
  - Current `/tmp/.X11-unix` is `nobody:nogroup` with mode `777`.
- `env DONT_PROMPT_WSL_INSTALL=1 xvfb-run -a npm test`: FAIL for the exact
  release gate.
  - The VS Code test runner reports `Exit code: 0`.
  - `xvfb-run` exits 1 because Xvfb cannot establish its listening sockets in
    this `/tmp/.X11-unix` state.
- `.github/workflows/ci.yml` extension-tests job now mirrors the release
  checklist in native Ubuntu CI:
  - install wasmtime for the selfhost wrapper used as `target/debug/arukellt`
  - `npm ci`
  - `npm run test:marketplace-metadata`
  - `npm run build`
  - `DONT_PROMPT_WSL_INSTALL=1 xvfb-run -a npm test`

Updated verdict: close-candidate `no`. Build, VSIX generation, marketplace
metadata, and non-Xvfb `npm test` are green with the WSL prompt suppressed, but
the exact release gate remains blocked by local Xvfb socket directory ownership.
The next useful evidence must come from that native Linux CI job, or from a local
environment where `/tmp/.X11-unix` is `root:root` with mode `1777`.

## Queue Move — 2026-05-17

Moved from `issues/open/` to `issues/blocked/`. Repo-side release prep for this
gate is complete: CI now installs wasmtime for the selfhost wrapper, then runs
`npm ci`, marketplace metadata validation, build, and
`DONT_PROMPT_WSL_INSTALL=1 xvfb-run -a npm test` on native Ubuntu. The only
remaining acceptance item requires native CI evidence or a local Xvfb socket
directory owned by `root:root` with mode `1777`.

## Recheck — 2026-05-17 late

- `/tmp/.X11-unix` state: `nobody:nogroup` with mode `777`.
- `xvfb-run -e /dev/stdout -a true`: **FAIL**.
  - Xvfb reports `/tmp/.X11-unix` must be owned by `root` and have mode `1777`.
- `env DONT_PROMPT_WSL_INSTALL=1 xvfb-run -e /dev/stdout -a npm test`: **FAIL**
  for the exact release gate in this local environment.
  - The VS Code test runner reports `Exit code: 0`.
  - `xvfb-run` exits 1 because Xvfb cannot establish listening sockets with the
    current `/tmp/.X11-unix` owner/mode.

Updated verdict: still blocked only on environment/native-CI evidence. The
extension test assertions are green under `vscode-test`, but this local
workspace cannot satisfy the exact `xvfb-run -a npm test` gate until Xvfb can
create its socket directory correctly.

## CI Close Evidence — 2026-05-17

- CI run: `25977063697`
  (`https://github.com/wogikaze/arukellt/actions/runs/25977063697`)
- Commit: `a4aa675af3ddc838ddbb2dec028fd8a58fc4b795`
- Run conclusion: **success**
- `VS Code extension tests` job `76358970846`: **PASS**
  - `Install extension dependencies`: PASS
  - `Check extension marketplace metadata`: PASS
  - `Build extension VSIX`: PASS
  - `Run extension tests`: PASS
- `Final Gate` job `76359022001`: **PASS**
  - Required CI layers included extension tests.
- `Component interop` job `76358984672`: **PASS**

Close verdict: **done**. The remaining local Xvfb issue is environment-specific
to this WSL checkout and is superseded by native Ubuntu CI evidence for the
release gate.

## Primary Paths

- `extensions/arukellt-all-in-one/` (extension source)
- Extension build configuration
- VSIX package output
- Activation test suite

## Non-Goals

- Cross-platform testing (Linux only for now)
- Performance optimization
