# AGENTS.md

This repository is the **Arukellt** language toolchain: compiler/runtime backends, standard library, CLI, tests, and documentation.

## Repository Boundary

This repo contains:

- compiler and runtime implementation under `crates/`
- standard library sources under `std/`
- CLI integration via selfhost wrapper (`scripts/run/arukellt-selfhost.sh`)
- fixture / benchmark / verification infrastructure under `tests/`, `benchmarks/`, and `scripts/`
- user-facing and design documentation under `docs/`

## Primary Source of Truth

Use these in order, depending on the question:

- **Current user-visible behavior**: `docs/current-state.md`
- **Current open work queue**: `issues/open/index.md`
- **Current dependency ordering**: `issues/open/dependency-graph.md`
- **Completed tracked work**: `issues/done/`
- **Design decisions / rationale**: `docs/adr/`
- **Verification contract**: `scripts/manager.py` (`python scripts/manager.py verify`)
- **Generated docs contract**: `scripts/gen/generate-docs.py`

## Current Work Surface

The active open queue is the generated issue index under `issues/open/`.
At the time of writing, the queue is centered on:

- WASI Preview 2 native component output
- `std::host::*` namespace rollout
- shared host capability facades across T1 / T3

Do not infer active work from old roadmap prose when `issues/open/index.md` disagrees.

## Documentation Rules

- Treat `docs/current-state.md` as the current behavior contract.
- Many landing pages are generated. After changing manual doc sources, regenerate docs with:

```bash
python3 scripts/gen/generate-docs.py
```

- Check for docs drift with:

```bash
python3 scripts/check/check-docs-consistency.py
```

- If queue structure changes, regenerate issue indexes with:

```bash
python3 scripts/gen/generate-issue-index.py
```

## Completion Criteria

Work is complete when the relevant scope is updated and verification passes.
For tracked issue work, that normally means:

1. `python scripts/manager.py verify` exits with status 0
2. generated artifacts touched by the work are regenerated and included
3. relevant docs / ADRs are updated when behavior changed
4. tracked issue files move from `issues/open/` to `issues/done/` when the task itself is completed
5. commits stay focused to the files changed for that task

## Verification Loop

- Quick pass: `python scripts/manager.py verify quick`
- Full pass: `python scripts/manager.py verify full`

## Tooling Notes

- Prefer `ig` for code search.
- Generated docs and manifest-backed stdlib reference pages should be regenerated, not hand-maintained.

## API Design Principles

The stdlib public surface is **trait-first / type-first**. User-reachable
free functions are **eradicated**, not kept as permanent bridges
(ADR-046, issue #709).

- **Require method syntax** `s.split(sep)`, `v.push(x)`, `n.to_string()` —
  do not ship `split(s, sep)`, `push(v, x)`, `i32_to_string(n)` as the
  lasting public API.
- **Prefer `impl Type` blocks** for new public APIs. Define methods on
  the type they operate on, not as standalone functions in a module.
- **Prefer associated functions** `Vec::new()`, `String::from("x")` over
  monomorphic constructors `Vec_new_i32()`, `String_from("x")`.
  No-receiver globals (`args`, `exit`, `println`) become associated
  functions on a namespace / handle type (e.g. `Env::args()`,
  `Process::exit(c)`), not permanent free functions.
- **Do not** leave public or prelude thin wrappers as the end state.
  Deprecated wrappers are migration-only (ADR-014 + W0009).
- **Exceptions**: compiler/runtime `__intrinsic_*` (manifest
  `kind = "intrinsic"`) only — never in the user-facing namespace.
  Private free helpers inside `std` are also forbidden in principle;
  use private methods or intrinsics.
- Monomorphic helpers (`*_i32`, `*_i64`, `*_f64`) are delete targets
  (#703), not user-facing API.

References: ADR-046 (free-function eradication), ADR-044 (trait/method
syntax), ADR-036 (trait-stdlib-redesign; D5 withdrawn), ADR-038
(operator overload traits), issue #709, issue #718, issue #703.

## Markdown Navigation

- When reading large Markdown files such as `README.md`, docs, ADRs, or issue indexes, prefer `markdive` over loading the whole file at once.
- Use `npx markdive` so the workflow works even when the CLI is not globally installed.
- Recommended flow:

```bash
npx markdive dive <file> --depth 2
npx markdive dive <file> --path <section-id> --depth 2
npx markdive read <file> --path <section-id>
```

- First inspect structure with `dive`, then drill down with `--path`, and only then read the target section with `read`.
- Fall back to normal file reads only when `markdive` cannot handle the document.
