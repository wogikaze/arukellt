# WORKBOARD

This file is the shared AI-managed task queue for the repository.
AI updates it; humans primarily read it.
It is the canonical place to park follow-up work, pick the next vertical slice, and record verified completion.

## Operating Rules

- Read this file before substantial planning or implementation work.
- Keep exactly one queue item in `Next`. If it becomes stale, promote the highest-priority unblocked item from `Ready`.
- Add newly discovered work to `Ready` unless a concrete dependency blocks it. Put dependency-gated work in `Blocked`.
- Keep task IDs stable as `WB-###`.
- Keep tasks as small vertical slices with one clear outcome.
- Move an item to `Done` only after the matching verification command or test has been run.
- When a task splits, add a follow-up item instead of mutating the old item beyond recognition.
- Keep `Done` entries concise and newest-first.
- Update this file in the same change when work starts, gets blocked, discovers follow-up tasks, or completes.

## Task Schema

Use this exact field order for every task:

### WB-000
title: Example task title
area: workflow
status: READY
priority: P2
owner: unassigned
depends_on: none
source: where this task came from
done_when:
- concrete verification outcome
notes:
- short context for future agents

Field rules:

- `status`: one of `NEXT`, `READY`, `BLOCKED`, `DONE`
- `priority`: one of `P0`, `P1`, `P2`, `P3`
- `owner`: `unassigned`, `ai`, or a short agent label
- `depends_on`: `none` or one or more `WB-###` identifiers
- `source`: file path, test name, user request, or other concrete origin
- `done_when`: 1 to 3 concrete checks
- `notes`: short bullets; newest note first if there are multiple notes

## Next

### WB-015

title: Make `chef test` print human-readable diagnostics on non-JSON compile failure
area: tooling
status: NEXT
priority: P1
owner: unassigned
depends_on: none
source: static CLI audit; `crates/chef/src/commands.rs`
done_when:

- `chef test <broken_file>` emits non-empty human-readable diagnostics before exiting non-zero
- regression tests cover the non-JSON compile-failure path
- `README.md` matches the verified `chef test` failure contract
notes:
- current non-JSON compile-failure path exits 1 silently while `--json` prints diagnostics
- `chef run` already emits diagnostics on compile failure, so the current `chef test` behavior is inconsistent within the same CLI

## Ready

### WB-016

title: Fix `chef benchmark` so `parse_success` is counted independently from typecheck success
area: tooling/benchmark
status: READY
priority: P1
owner: unassigned
depends_on: none
source: static CLI audit; `crates/chef/src/benchmark.rs`
done_when:

- `parse_success` increments for parse-only success even when typechecking fails
- regression tests cover parse-success/typecheck-failure and full-success cases
- benchmark output semantics are documented or clarified in repo docs
notes:
- current logic makes `parse_success` and `typecheck_success` always equal
- the bug hides the distinction between parser coverage and typecheck coverage

### WB-017

title: Prefer compile diagnostics over unsupported-format errors in `arktdoc`
area: tooling/docs
status: READY
priority: P2
owner: unassigned
depends_on: none
source: static CLI audit; `crates/arktdoc/src/main.rs`
done_when:

- `arktdoc <broken_file> --format markdown` reports compile failure rather than masking it with a format error
- regression tests cover broken-source plus unsupported-format ordering
- `arktdoc` error precedence is deliberate and documented by tests
notes:
- `--format markdown` is already rejected explicitly for valid input
- the remaining issue is error ordering when the source itself is broken

### WB-018

title: Document and test the no-`--output` contract for `arktc build`
area: tooling/docs
status: READY
priority: P2
owner: unassigned
depends_on: none
source: static CLI audit; `arktc build` behavior; missing README/test coverage
done_when:

- tests lock the observed behavior when `arktc build` succeeds without `--output`
- `README.md` states whether output is intentionally discarded or should be rejected
- the CLI contract is explicit rather than implicit
notes:
- current behavior appears to succeed while silently discarding generated WASM bytes
- this may be acceptable, but the contract is currently undocumented

### WB-007

title: Add a browser-level smoke path for the static docs app shell
area: docs
status: READY
priority: P3
owner: unassigned
depends_on: none
source: `docs/index.html`; `docs/app.js`; `crates/arktc/tests/docs_site.rs`
done_when:

- a repeatable smoke command validates `#/language-tour` and `#/std`
- the smoke path is documented in repo contributor docs
- the check can fail without needing manual browser inspection
notes:
- deprioritized behind newly found CLI contract issues
- current docs-site tests lock the static contract and asset paths, but not route rendering in a browser runtime

## Blocked

### WB-008

title: Record a deployed GitHub Pages smoke URL for the docs shell
area: docs/release
status: BLOCKED
priority: P3
owner: unassigned
depends_on: repo-level GitHub Pages configuration
source: docs app shell exists, but deployment settings live outside the workspace
done_when:

- Pages source is configured
- the deployed URL is documented in the repo
- a smoke pass is recorded against the deployed site
notes:
- blocked on repository settings rather than code in this worktree

## Done

### WB-014

title: Make CLI failure paths non-silent and keep formatter writes non-destructive on invalid input
area: tooling
status: DONE
priority: P1
owner: ai
depends_on: none
source: follow-up after WB-011; `crates/chef/src/commands.rs`; `crates/arktfmt/src/main.rs`
done_when:

- `chef test` prints human-readable compile-failure diagnostics when `--json` is not used
- `arktfmt` rejects invalid source, including lexer/parser errors, and `--write` does not corrupt the source file
- regression tests and `README.md` reflect the verified contract
notes:
- verified with `cargo fmt --all --check`, `cargo test -p chef --test cli -p arktfmt --test cli`, and `cargo test`
- `chef` no longer fails compile errors silently on the non-JSON path
- `arktfmt` now explicitly rejects invalid input and keeps `--write` non-destructive on parse failure

### WB-011

title: Deepen `--help` coverage for all public binaries and subcommands
area: docs/tooling
status: DONE
priority: P2
owner: ai
depends_on: none
source: user CLI audit; `arktc`; `chef`; `arktfmt`; `arktdoc`; `arktup`
done_when:

- every public binary and subcommand has a tested `--help` path
- repo docs include at least one concrete command example per public binary
- help text mentions important prototype constraints where behavior is intentionally limited
notes:
- verified with `cargo fmt --all --check`, `cargo test -p arktc --test help -p chef --test help -p arktfmt --test help -p arktdoc --test help -p arktup --test help`, and `cargo test`
- added `help.rs` regression coverage for every public binary and public subcommand across `arktc`, `chef`, `arktfmt`, `arktdoc`, and `arktup`
- help text now explains the intentionally narrow prototype contract, including the limited WASM subset, `arktdoc` JSON-only output, `chef test` snapshot/json modes, and `arktup` local-state-only behavior
