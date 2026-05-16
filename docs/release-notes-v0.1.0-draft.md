# Arukellt v0.1.0 Draft Release Notes

This draft is the repository-side release note source for the next tagged
release. Attach or adapt it when creating the GitHub release.

## Highlights

- Selfhost compiler/runtime path is the current product surface, with the Rust
  CLI acting as a thin host runner for the selfhost Wasm compiler.
- `wasm32-wasi-p1` remains the stable default target; `wasm32-wasi-p2` remains
  the primary component / GC-native target.
- Verification now reports categorized results through `python3 scripts/manager.py verify quick`.
- The VS Code extension package includes marketplace metadata, icons,
  release procedure documentation, task execution coverage, test discovery
  coverage, and failure-recovery E2E coverage.
- Package workspace scripts support `arukellt script list`, JSON script
  listing, and `arukellt script run <name> [args...]` with project-root
  execution.
- Parser diagnostics now warn on deprecated `import <name>` syntax and offer
  `use <name>` as the replacement.
- Benchmarks include a dedicated I/O-heavy `file_io` workload exercising
  repeated host filesystem write/read calls.

## Known Limitations

- Full selfhost bootstrap / parity gates still have open blockers in the issue
  queue.
- Exact headless extension activation via `xvfb-run -a npm test` requires a
  correctly configured local or CI Xvfb environment.
- Formatter CLI/LSP parity remains open until the active formatter surface is
  available on both sides.
- GitHub release publication, tag/version matching, and release-note attachment
  remain manual release steps.

## Verification Snapshot

- `python3 scripts/manager.py verify quick`: 22/22 checks pass.
- `npm test` in `extensions/arukellt-all-in-one`: passes in the local extension
  test harness.
- `mise bench` and `mise bench:update-baseline`: complete and include
  `file_io`; benchmark result rows preserve existing per-workload failures and
  skips rather than masking them.
