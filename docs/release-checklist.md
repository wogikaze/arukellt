# Release Checklist

> **Structured SSOT:** [`data/release-guarantees.toml`](data/release-guarantees.toml)
> (generated matrix: [`data/release-guarantees.md`](data/release-guarantees.md)).
> Normative tiers: [`release-criteria.md`](release-criteria.md).

This checklist is executed before every tagged release.

## Release-blocking checks

The release-blocker set is generated solely from `release-guarantees.toml`
checks with `release_blocking = true`. No supplemental lists.

**Failing checks vs distinct incidents:** A single incident may be tracked by
multiple checks. The `incident_id` field links checks to the underlying
failure incident. When counting blockers, count distinct `incident_id` values,
not individual checks.

<!-- BEGIN GENERATED:release-blockers -->
<!-- Generated from docs/data/release-guarantees.toml; do not edit this block. -->
- [ ] **CI `check_compile_wasm32_gc`** — `scripts/run/arukellt-selfhost.sh compile docs/examples/hello.ark --target wasm32-gc -o .build/release-checks/wasm32-gc.wasm` (job: `verification`)
- [ ] **CI `check_compile_wasm32`** — `scripts/run/arukellt-selfhost.sh compile docs/examples/hello.ark --target wasm32 -o .build/release-checks/wasm32.wasm` (job: `verification`)
- [ ] **CI `check_run_wasmtime`** — `scripts/run/arukellt-selfhost.sh run tests/fixtures/hello_world.ark` (job: `verification`)
- [ ] **CI `check_fixture_harness`** [FAIL] — `python3 scripts/manager.py verify fixtures` (job: `verification`)
- [ ] **CI `check_determinism`** — `bash scripts/check/check-release-determinism.sh` (job: `verification`)
- [ ] **CI `check_no_panic`** — `bash scripts/check/check-panic-audit.sh` (job: `verification`)
- [ ] **CI `check_cli_check`** — `python3 scripts/check/check-cli-guarantees.py check` (job: `verification`)
- [ ] **CI `check_cli_init`** — `python3 scripts/check/check-init-templates.py` (job: `verification`)
- [ ] **CI `check_cli_doc`** — `python3 scripts/check/check-manifest-doc.py` (job: `verification`)
- [ ] **CI `check_cli_help`** — `python3 scripts/check/check-cli-guarantees.py help` (job: `verification`)
- [ ] **CI `check_close_gate_076`** [FAIL] — `python3 scripts/check/check-false-done-close-gates.py` (job: `verification`)
- [ ] **CI `check_selfhost_fixpoint`** [FAIL] — `python3 scripts/manager.py selfhost fixpoint --build` (job: `verification`)
- [ ] **CI `check_selfhost_cli_parity`** [FAIL] — `python3 scripts/manager.py selfhost parity --mode --cli` (job: `verification`)
- [ ] **CI `check_selfhost_diag_parity`** — `python3 scripts/manager.py selfhost diag-parity` (job: `verification`)
- [ ] **CI `check_wat_roundtrip`** [FAIL] — `bash scripts/run/wat-roundtrip.sh` (job: `verification`)
- [ ] **CI `check_component_interop_wasmtime`** [FAIL] — `python3 scripts/manager.py verify --component-interop` (job: `verification`)
<!-- END GENERATED:release-blockers -->

## Non-blocking informational checks

These checks run in CI but do not block a tagged release. See the full
check catalogue in [`data/release-guarantees.md`](data/release-guarantees.md)
for `release_blocking = false` entries (opt-equivalence, binary smoke,
LSP, ark.toml, DAP, VS Code DAP).

## Non-blocking packaging checklist

The following items are **not** release blockers. They are packaging and
distribution steps that should be completed before publishing a release
artifact, but failure here does not invalidate the release contract.

### Binary distribution

- [ ] **CI** — Release CLI entrypoint prepared on linux-x86_64 runner
- [ ] **CI** — Release binary size < 50 MB
- [ ] **CI** — SHA256 checksum generated alongside binary
- [ ] **CI on tag** — Version in `docs/data/project-state.toml` matches the tag

### Extension distribution

- [ ] **CI** — `cd extensions/arukellt-all-in-one && npm ci && npm run build` succeeds
- [ ] **CI** — VSIX package generated (`.vsix` file exists)
- [ ] **CI** — Extension activation tests pass (`xvfb-run -a npm test`)
- [ ] **CI** — Packaged VSIX installs in VS Code and activates without errors (`xvfb-run -a npm run test:vsix-live`)
- [ ] **CI** — LSP connects and shows "Ready" in language status through the installed VSIX
- [ ] **CI** — LSP protocol compliance verified via E2E tests (initialize, shutdown, completion, hover, definition)
- [ ] **CI** — Diagnostics appear on save for a file with type errors through the installed VSIX
- [ ] **CI** — Completion, hover, and go-to-definition work in live editor through the installed VSIX

### Failure recovery

- [ ] **CI** — After killing the LSP process, the extension shows "Error" status
- [ ] **CI** — `Arukellt: Restart Language Server` command restarts successfully
- [ ] **CI** — Invalid `arukellt.server.path` setting shows a clear error message
- [ ] **CI** — Missing `ark.toml` gracefully falls back to single-file mode

## Post-release

- [ ] **Manual** — Release notes written and attached to GitHub release
- [ ] **Manual** — `docs/current-state.md` reflects shipped capabilities
