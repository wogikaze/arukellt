# Release Checklist

This checklist is executed before every tagged release.
Items marked **CI** are automated; items marked **Manual** require human verification.

## Pre-release

- [ ] **CI** — `cargo test --workspace --exclude ark-llvm` passes
- [ ] **CI** — `cargo test -p arukellt --test harness` passes (all fixtures green)
- [ ] **CI** — `cargo clippy --workspace --exclude ark-llvm -- -D warnings` clean
- [ ] **CI** — `cargo fmt --all -- --check` clean
- [ ] **CI** — `bash scripts/run/verify-harness.sh --quick` passes
- [ ] **CI** — `bash scripts/run/verify-harness.sh --component` passes (component interop)
- [ ] **CI** — `bash scripts/run/verify-harness.sh --opt-equiv` passes (O0 == O1)
- [ ] **CI** — Binary smoke: `arukellt --version` exits 0
- [ ] **CI** — Binary smoke: `arukellt run tests/fixtures/hello_world.ark` outputs `Hello, World!`
- [ ] **CI** — Binary smoke: `arukellt check tests/fixtures/type_error.diag` exits non-zero
- [ ] **CI** — Determinism: same source produces identical `.wasm` across two builds
- [ ] **CI** — Selfhost: `bash scripts/run/verify-bootstrap.sh --stage1-only` passes
- [ ] **CI** — LSP unit tests: `cargo test -p ark-lsp --lib` passes
- [ ] **CI** — LSP E2E tests: `cargo test -p ark-lsp --test lsp_e2e -- --test-threads=1` passes

## Binary distribution

- [ ] **CI** — Release binary built for linux-x86_64
- [ ] **CI** — Release binary size < 50 MB
- [ ] **CI** — SHA256 checksum generated alongside binary
- [ ] **Manual** — Version in `Cargo.toml` matches the tag

## Extension distribution

- [ ] **CI** — `cd extensions/arukellt-all-in-one && npm ci && npm run build` succeeds
- [ ] **CI** — VSIX package generated (`.vsix` file exists)
- [ ] **CI** — Extension activation tests pass (`xvfb-run -a npm test`)
- [ ] **Manual** — VSIX installs in VS Code and activates without errors
- [ ] **Manual** — LSP connects and shows "Ready" in language status
- [ ] **CI** — LSP protocol compliance verified via E2E tests (initialize, shutdown, completion, hover, definition)
- [ ] **Manual** — Diagnostics appear on save for a file with type errors
- [ ] **CI** — Formatter output matches between CLI and LSP (shared `format_source()`)
- [ ] **Manual** — Completion, hover, and go-to-definition work in live editor

## Failure recovery

- [ ] **Manual** — After killing the LSP process, the extension shows "Error" status
- [ ] **Manual** — `Arukellt: Restart Language Server` command restarts successfully
- [ ] **Manual** — Invalid `arukellt.path` setting shows a clear error message
- [ ] **Manual** — Missing `ark.toml` gracefully falls back to single-file mode

## Post-release

- [ ] **Manual** — Release notes written and attached to GitHub release
- [ ] **Manual** — `CHANGELOG.md` updated in extension
- [ ] **Manual** — `docs/current-state.md` reflects shipped capabilities
