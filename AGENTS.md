# AGENTS

This file describes how to work safely in the Arukellt repository.

## Scope

Arukellt is an experimental Rust workspace for an LLM-first language and toolchain. The codebase is small, but the crates are intentionally separated by pipeline stage:

- `lang-core`: syntax, AST, diagnostics, and typechecking
- `lang-ir`: IR lowering
- `lang-interp`: execution on High IR
- `lang-backend-wasm`: WASM emission
- `arktc`: compiler-facing public binary
- `chef`: test, run, and benchmark public binary
- `arktfmt`: formatter public binary
- `lang-playground-core`: browser-facing JSON and wasm-bindgen wrapper

When you change behavior, keep that layering intact. Do not make public binary code become the source of truth for compiler behavior.

## Workflow

- Work from the workspace root: `/home/wogikaze/arukellt/.worktrees/arukellt-v0`
- Read `issues/index.md` before substantial work and treat it as the shared AI-managed queue
- Prefer small vertical slices over large speculative rewrites
- Add or update tests before changing behavior
- Keep diagnostics stable once a test depends on a diagnostic code
- Prefer interpreter-first validation before extending WASM codegen
- When you discover follow-up work or finish a queued task, update `issues/index.md` and the matching issue file in the same change

## Verification

Before claiming work is complete, run the full suite:

```bash
cargo fmt
cargo test
```

If you change benchmark behavior or CLI output relevant to evaluation, also run:

```bash
cargo run -p chef -- benchmark benchmarks/pure_logic.json
```

Do not claim a backend feature works unless you ran the matching test or command after the change.

## Conventions

- Blocks are indentation-sensitive in the language; parser changes should preserve tolerant recovery behavior
- The top-level ordering rule is deliberate: `import capability` -> `type` -> `fn`
- `if` must keep mandatory `else`
- `match` should remain exhaustive, with `_` allowed only as the last arm and warned otherwise
- `null` stays forbidden
- Capability calls from pure functions are compile errors, even if internal recovery continues
- Structured diagnostics must keep schema version `v0.1` unless intentionally revised everywhere

## Current Boundaries

Respect the current prototype boundaries unless the task explicitly expands them:

- The interpreter supports more of the language than the WASM backend
- The WASM backend currently handles the pure scalar subset only
- `lang-playground-core` is for parse/typecheck/run loops, not full project builds
- Standard library work is mostly still ahead; do not invent broad APIs casually

## Good Extension Order

If you need to add capability, do it in this order:

1. Add a failing test in the owning crate
2. Extend `lang-core` if syntax or typing changes
3. Extend `lang-ir` if a new runtime representation is needed
4. Extend `lang-interp` so the development loop stays usable
5. Extend `arktc` for compiler-facing behavior
6. Extend `chef` for test/run/benchmark behavior
7. Extend `arktfmt` for formatting behavior
8. Extend `lang-backend-wasm` if the feature is meant to compile to WASM now
9. Extend `lang-playground-core` only if browser consumers need the new surface

## Avoid

- Hiding compiler logic inside the CLI
- Adding implicit conversions or fallback behavior that weakens diagnostics
- Expanding the language surface without matching tests
- Claiming WASM parity when only interpreter behavior was tested
