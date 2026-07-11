# Release Checklist

> **Structured SSOT:** [`data/release-guarantees.toml`](data/release-guarantees.toml)
> (generated matrix: [`data/release-guarantees.md`](data/release-guarantees.md)).
> Normative tiers: [`release-criteria.md`](release-criteria.md).

This checklist is executed before every tagged release.
Items marked **CI** are automated; items marked **CI on tag** run for tagged
release refs; items marked **Manual** require human verification.

## Pre-release

- [ ] **CI** — `python3 scripts/manager.py verify` passes
- [ ] **CI** — `arukellt compile docs/examples/hello.ark --target wasm32-gc` produces valid Wasm
- [ ] **CI** — `arukellt compile docs/examples/hello.ark --target wasm32` produces valid Wasm
- [ ] **CI** — `python3 scripts/manager.py verify fixtures` passes for the observed harness snapshot
- [ ] **CI** — `bash scripts/check/check-panic-audit.sh` passes
- [ ] **CI** — `python3 scripts/manager.py selfhost fixpoint` passes
- [ ] **CI** — `python3 scripts/manager.py selfhost fixture-parity` passes
- [ ] **CI** — `python3 scripts/manager.py selfhost parity --mode --cli` passes
- [ ] **CI** — `python3 scripts/manager.py selfhost diag-parity` passes
- [ ] **CI** — opt-equivalence (O0 == O1): `bash scripts/run/test-opt-equivalence.sh --quick` passes
- [ ] **CI** — Binary smoke: `arukellt --version` exits 0
- [ ] **CI** — Binary smoke: `arukellt run tests/fixtures/hello_world.ark` outputs `Hello, World!`
- [ ] **CI** — Binary smoke: `arukellt check tests/fixtures/type_error.diag` exits non-zero
- [ ] **CI** — Determinism: same source produces identical `.wasm` across two builds
<!-- DEFERRED: legacy `bash scripts/run/verify-bootstrap.sh --stage1-only` uses the retired Rust bootstrap path and currently cannot parse the selfhost source surface. Current selfhost release coverage is `python scripts/manager.py verify fixtures` plus `python scripts/manager.py verify --selfhost-parity`. -->
- [ ] **CI** — Selfhost LSP: `python scripts/manager.py verify quick` LSP gates (#568, #569) pass
  (Rust `ark-lsp` crate retired in #572; selfhost `src/compiler/lsp.ark` via `arukellt lsp` is the source of truth.)

## Binary distribution

- [ ] **CI** — Release CLI entrypoint prepared on linux-x86_64 runner
- [ ] **CI** — Release binary size < 50 MB
- [ ] **CI** — SHA256 checksum generated alongside binary
- [ ] **CI on tag** — Version in `docs/data/project-state.toml` matches the tag

## Extension distribution

- [ ] **CI** — `cd extensions/arukellt-all-in-one && npm ci && npm run build` succeeds
- [ ] **CI** — VSIX package generated (`.vsix` file exists)
- [ ] **CI** — Extension activation tests pass (`xvfb-run -a npm test`)
- [ ] **CI** — Packaged VSIX installs in VS Code and activates without errors (`xvfb-run -a npm run test:vsix-live`)
- [ ] **CI** — LSP connects and shows "Ready" in language status through the installed VSIX
- [ ] **CI** — LSP protocol compliance verified via E2E tests (initialize, shutdown, completion, hover, definition)
- [ ] **CI** — Diagnostics appear on save for a file with type errors through the installed VSIX
- [ ] **CI** — Completion, hover, and go-to-definition work in live editor through the installed VSIX

## Failure recovery

- [ ] **CI** — After killing the LSP process, the extension shows "Error" status
- [ ] **CI** — `Arukellt: Restart Language Server` command restarts successfully
- [ ] **CI** — Invalid `arukellt.server.path` setting shows a clear error message
- [ ] **CI** — Missing `ark.toml` gracefully falls back to single-file mode

## Post-release

- [ ] **Manual** — Release notes written and attached to GitHub release
- [ ] **Manual** — `docs/current-state.md` reflects shipped capabilities
