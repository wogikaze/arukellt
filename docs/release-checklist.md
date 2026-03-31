# Release Checklist

This checklist is executed before every tagged release.
Items marked **CI** are automated; items marked **Manual** require human verification.

## Pre-release

- [ ] **CI** — `cargo test --workspace --exclude ark-llvm --exclude ark-lsp` passes
- [ ] **CI** — `cargo test -p arukellt --test harness` passes (all fixtures green)
- [ ] **CI** — `cargo clippy --workspace --exclude ark-llvm --exclude ark-lsp -- -D warnings` clean
- [ ] **CI** — `cargo fmt --all -- --check` clean
- [ ] **CI** — `bash scripts/verify-harness.sh --quick` passes
- [ ] **CI** — `bash scripts/verify-harness.sh --component` passes (component interop)
- [ ] **CI** — `bash scripts/verify-harness.sh --opt-equiv` passes (O0 == O1)
- [ ] **CI** — Binary smoke: `arukellt --version` exits 0
- [ ] **CI** — Binary smoke: `arukellt run tests/fixtures/hello_world.ark` outputs `Hello, World!`
- [ ] **CI** — Binary smoke: `arukellt check tests/fixtures/type_error.diag` exits non-zero
- [ ] **CI** — Determinism: same source produces identical `.wasm` across two builds
- [ ] **CI** — Selfhost: `bash scripts/verify-bootstrap.sh --stage1-only` passes

## Binary distribution

- [ ] **CI** — Release binary built for linux-x86_64
- [ ] **CI** — Release binary size < 50 MB
- [ ] **CI** — SHA256 checksum generated alongside binary
- [ ] **Manual** — Version in `Cargo.toml` matches the tag

## Extension distribution

- [ ] **CI** — `cd extensions/arukellt-all-in-one && npm ci && npm run build` succeeds
- [ ] **CI** — VSIX package generated (`.vsix` file exists)
- [ ] **Manual** — VSIX installs in VS Code and activates without errors
- [ ] **Manual** — LSP connects and shows "Ready" in language status
- [ ] **Manual** — Diagnostics appear on save for a file with type errors
- [ ] **Manual** — Completion, hover, and go-to-definition work

## Failure recovery

- [ ] **Manual** — After killing the LSP process, the extension shows "Error" status
- [ ] **Manual** — `Arukellt: Restart Language Server` command restarts successfully
- [ ] **Manual** — Invalid `arukellt.path` setting shows a clear error message
- [ ] **Manual** — Missing `ark.toml` gracefully falls back to single-file mode

## Post-release

- [ ] **Manual** — Release notes written and attached to GitHub release
- [ ] **Manual** — `CHANGELOG.md` updated in extension
- [ ] **Manual** — `docs/current-state.md` reflects shipped capabilities
