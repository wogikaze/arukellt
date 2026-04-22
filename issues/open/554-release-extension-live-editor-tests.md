# Release: Extension Live Editor Tests

> **Status:** open
> **Track:** release
> **Type:** Verification

## Scope

Ensure extension features work in live editor for release verification.

## Checklist Source

docs/release-checklist.md — Extension distribution section

## Acceptance

- [ ] VSIX installs in VS Code and activates without errors
- [ ] LSP connects and shows "Ready" in language status
- [ ] Diagnostics appear on save for a file with type errors
- [ ] Completion, hover, and go-to-definition work in live editor

## Required Verification

- Install VSIX in VS Code
- Verify extension activation
- Test LSP connection status
- Test diagnostics on type error file
- Test completion, hover, and go-to-definition in live editor

## Close Gate

All extension features must work correctly in live editor environment.

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

- A separate Code/VS Code instance is already running in this environment, so the extension test harness cannot claim the instance lock.
- No live-editor assertions ran, so the manual release checks in this issue remain unrunnable here until the existing Code session is closed or the tests are rerun on a clean machine.

## Verdict

Close-candidate status: not runnable in the current environment because the VS Code test harness is blocked by an existing Code instance, even though VSIX packaging itself is runnable.
