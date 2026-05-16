---
Status: blocked
Updated: 2026-05-17
ID: 554
Track: release
Type: Verification
Depends on: none
Blocked by: "manual live VS Code verification with the packaged VSIX"
---

# Release: Extension Live Editor Tests

> **Status:** blocked
> **Track:** release
> **Type:** Verification

## Scope

Ensure extension features work in live editor for release verification.

## Checklist Source

docs/release-checklist.md — Extension distribution section

## Acceptance

- [ ] VSIX installs in VS Code and activates without errors — Manual
- [ ] LSP connects and shows "Ready" in language status — Manual
- [ ] Diagnostics appear on save for a file with type errors — Manual
- [ ] Completion, hover, and go-to-definition work in live editor — Manual

## Required Verification

- Install VSIX in VS Code
- Verify extension activation
- Test LSP connection status
- Test diagnostics on type error file
- Test completion, hover, and go-to-definition in live editor

## Close Gate

All extension features must work correctly in live editor environment.

## Assessment — 2026-05-16

### Acceptance Criteria Status

| Criterion | Type | Status | Evidence |
|-----------|------|--------|----------|
| VSIX installs and activates | Manual | UNVERIFIED | VSIX exists and is buildable; installation requires interactive VS Code |
| LSP shows "Ready" status | Manual | UNVERIFIED | Requires VSIX installed in live VS Code |
| Diagnostics on type-error save | Manual | UNVERIFIED | Requires live editor interaction |
| Completion/hover/definition work | Manual | UNVERIFIED | Requires live editor interaction |

### Analysis

- **All four acceptance criteria are inherently manual.** None can be verified from this headless environment.
- **The VSIX packaging pipeline is confirmed working:** `npm run build` succeeds, produces `arukellt-all-in-one-0.0.1.vsix` (327 files, 485.26 KB). This is the prerequisite for manual verification.
- **The E2E test suite** (`src/test/extension.test.js`) covers definition, hover, diagnostics, and debug behavior programmatically via `LspPipeSession` (direct JSON-RPC to `arukellt lsp`) and `vscode.*` API calls. These provide CI-level coverage for the same features. The live editor checks are a complementary manual layer.
- **Environment blockers remain:** local Xvfb cannot start because `/tmp/.X11-unix`
  has the wrong owner/mode in this environment. The WSL VS Code detection prompt
  can be suppressed with `DONT_PROMPT_WSL_INSTALL=1`, and non-Xvfb `npm test`
  now exits 0, but the exact `xvfb-run -a npm test` release gate still exits 1.

### Recommendation

**Do not close.** This is a fully manual verification issue that requires:

1. A native Linux or macOS desktop with VS Code installed (not WSL).
2. Install the VSIX: `code --install-extension extensions/arukellt-all-in-one/arukellt-all-in-one-0.0.1.vsix`
3. Verify all four acceptance criteria interactively.

Consider that the E2E test suite in `src/test/extension.test.js` already provides automated CI coverage for the LSP protocol features (definition, hover, diagnostics). The live editor manual checks serve as a release confidence gate. If CI E2E tests pass reliably, this issue's criteria may be candidates for downgrading from release-blocker to release-advisory.

## Primary Paths

- Extension installation process
- LSP connection handling
- Diagnostic display
- Code intelligence features (completion, hover, go-to-definition)

## Non-Goals

- Performance optimization
- Cross-editor testing (VS Code only)

## Verification Evidence

- Environment probe:
  - `which code` -> `/home/wogikaze/.vscode-server/bin/560a9dba96f961efea7b1612916f89e5d5d4d679/bin/remote-cli/code`
  - `which xvfb-run` -> `/usr/bin/xvfb-run`
  - `which npm` -> `/home/wogikaze/.nvm/versions/node/v23.6.0/bin/npm`
  - `which node` -> `/home/wogikaze/.nvm/versions/node/v23.6.0/bin/node`
  - `DISPLAY=:0`
  - `WAYLAND_DISPLAY=wayland-0`
- Extension package scripts present in `extensions/arukellt-all-in-one/package.json`:
  - `build` -> `npm run package`
  - `package` -> `vsce package`
  - `test` -> `vscode-test`
- `cd extensions/arukellt-all-in-one && npm run build`:
  - PASS
  - Produced `extensions/arukellt-all-in-one/arukellt-all-in-one-0.0.1.vsix`
- `cd extensions/arukellt-all-in-one && xvfb-run -a npm test`:
  - FAIL
  - `vscode-test` launched the VS Code binary from `.vscode-test/vscode-linux-x64-1.116.0`
  - Harness stopped with `Running extension tests from the command line is currently only supported if no other instance of Code is running.`

## Blockers

- Local Xvfb cannot start because `/tmp/.X11-unix` is not `root:root` with mode
  `1777` in this environment.
- No live-editor assertions ran, so the manual release checks in this issue
  remain unrunnable here until the VSIX is tested in a native VS Code desktop
  session or a native Linux CI environment.

## Verdict

Close-candidate status: not runnable in the current environment because local
Xvfb is blocked by `/tmp/.X11-unix` ownership/mode, even though VSIX packaging
and non-Xvfb extension tests are runnable.

## Recheck — 2026-05-14

- VSIX package exists: `extensions/arukellt-all-in-one/arukellt-all-in-one-0.0.1.vsix`
  (`498414` bytes).
- `npm test` in `extensions/arukellt-all-in-one/`: PASS in the local extension
  harness (`vscode-test` reports `Exit code: 0`).
- `xvfb-run -a npm test`: still not usable in this workspace, but the current
  blocker is Xvfb startup rather than an extension assertion:
  - `xvfb-run -a true` exits 1.
  - Xvfb reports `/tmp/.X11-unix` should be owned by root.
  - `sudo chown root:root /tmp/.X11-unix` could not be run here because sudo
    requires a password.

Updated close-candidate status: still `no`. This issue requires manual
live-editor checks in an actual VS Code session, including diagnostics and
completion/hover/definition behavior after installing the VSIX.

## Recheck — 2026-05-16

- VSIX package exists: `extensions/arukellt-all-in-one/arukellt-all-in-one-0.0.1.vsix`
  (327 files, 485.26 KB). Build and packaging confirmed PASS.
- Runtime/CI blockers:
  - `xvfb-run -a npm test`: cannot run in this WSL environment because the
    VS Code test harness displays an interactive WSL detection prompt
    ("To use Visual Studio Code with the Windows Subsystem for Linux...").
  - `npm test` (without xvfb): shows 9 assertion failures (Debug Launch, Go to
    Definition range, Hover content) — some or all of these may be environment
    related.
- All four acceptance items require manual verification in a native VS Code
  session (outside WSL) with the VSIX installed:
  1. VSIX installs and activates without errors
  2. LSP connects and shows "Ready" in language status
  3. Diagnostics appear on save for a file with type errors
  4. Completion, hover, and go-to-definition work in live editor

Updated close-candidate status: still `no`. This is a fully manual verification
issue that cannot be closed from this environment. The VSIX is confirmed buildable
and packagable; the remaining checks require interactive VS Code sessions.

## Recheck — 2026-05-17

- `npm ci` in `extensions/arukellt-all-in-one/`: PASS (392 packages installed).
- `npm run build` in `extensions/arukellt-all-in-one/`: PASS.
  - VSIX written to `extensions/arukellt-all-in-one/arukellt-all-in-one-0.0.1.vsix`
    (327 files, 485.26 KB).
- `npm run test:marketplace-metadata`: PASS (`marketplace metadata OK`).
- `env DONT_PROMPT_WSL_INSTALL=1 npm test`: PASS (`Exit code: 0`).
- `env DONT_PROMPT_WSL_INSTALL=1 xvfb-run -a npm test`: FAIL for the exact
  release gate.
  - The VS Code test runner reports `Exit code: 0`.
  - `xvfb-run` exits 1 because Xvfb reports `/tmp/.X11-unix` must be owned by
    `root` and have mode `1777`; the current directory is `nobody:nogroup` with
    mode `777`.
- Static/E2E coverage inventory:
  - `src/test/extension.test.js` contains automated assertions for extension
    activation, language status `$(check) Ready`, diagnostics stability,
    completion, hover, go-to-definition, restart, failure recovery, task
    provider, and project tree behavior.
  - Those automated tests cannot be used as a substitute for this issue's manual
    live-editor acceptance until they pass in native Linux CI or a clean
    non-WSL VS Code environment.

Updated close-candidate status: still `no`. The VSIX build prerequisite and
non-Xvfb automated tests are green, but all four acceptance criteria remain
manual and unverified in this environment.

## Queue Move — 2026-05-17

Moved from `issues/open/` to `issues/blocked/`. The automated prerequisites are
green (`npm ci`, VSIX build, marketplace metadata, and non-Xvfb extension tests
with the WSL prompt suppressed), but all acceptance criteria are manual live
VS Code checks that require installing the packaged VSIX in an interactive
editor session.
