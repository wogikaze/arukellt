# Release: Extension Activation Tests

> **Status:** open
> **Track:** release
> **Type:** Verification

## Scope

Ensure extension activation tests pass for release verification.

## Checklist Source

docs/release-checklist.md — Extension distribution section

## Acceptance

- [ ] `cd extensions/arukellt-all-in-one && npm ci && npm run build` succeeds
- [ ] VSIX package generated (`.vsix` file exists)
- [ ] Extension activation tests pass (`xvfb-run -a npm test`)

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

## Required Verification

- Build extension from source
- Verify VSIX package generation
- Run activation tests in headless X environment

## Close Gate

Extension build, VSIX generation, and activation tests must all pass.

## Primary Paths

- `extensions/arukellt-all-in-one/` (extension source)
- Extension build configuration
- VSIX package output
- Activation test suite

## Non-Goals

- Cross-platform testing (Linux only for now)
- Performance optimization
