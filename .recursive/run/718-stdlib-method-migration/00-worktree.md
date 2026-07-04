# Phase 0 — Worktree Setup (run 718-stdlib-method-migration)

## Worktree Location

- **Location**: `.worktrees/718-stdlib-method-migration`
- **Branch**: `recursive/718-stdlib-method-migration`
- **Base commit**: `b53ebf271` (docs(agents): add API design principles)
- **gitignore**: `.worktrees/` is in `.gitignore` (verified)

## Setup Commands

- `git worktree add .worktrees/718-stdlib-method-migration -b recursive/718-stdlib-method-migration`
- No build step needed (Arukellt uses selfhost wasm bootstrap; no cargo build required for stdlib work)

## Baseline Test

- **Command**: `python scripts/manager.py verify quick`
- **Result**: Exit code 1 — pre-existing failures (not caused by this run)

### Pre-existing failures (acknowledged, not blocking)

| Gate | Category | Notes |
|------|----------|-------|
| false-done close-gate #657 | verification-hygiene | sockets gate timeout (120s) |
| doc example check | verification-hygiene | ADR-040 file open errors (6 blocks) |
| runtime Wasm debug smoke (#638) | verification-hygiene | cargo build tools/host-linker failure |
| LSP performance smoke (#463) | editor-tooling | LSP exits 134 (5 failures) |
| GC array smoke gate | verification-hygiene | wasm out-of-bounds memory access |
| selfhost LSP lifecycle (#569) | editor-tooling | LSP exits 134 (21 failures) |
| internal link integrity | verification-hygiene | 2 broken links in gc-implementation-plan.md |
| selfhost formatter parity (#216) | verification-hygiene | selfhost crash |

### Passing gates (subset)

- Fixture manifest completeness (2671 entries)
- Language specification OK
- stdlib manifest check
- Stdlib specification OK
- Documentation structure OK
- All required ADRs decided
- docs freshness
- issues/done/ hygiene
- repository structure
- selfhost analysis API gate (#568)
- generated file boundary check
- WIT bindings round-trip (#618)
- component standard-world gate (#118)
- compiler boundary check
- host_stub compile gate (#292)
- in-file test adoption report (#715)
- WASI P1 syscall registry (#073)
- Wasm micro features
- LSP standard completeness (#219)
- LSP code actions (#217)
- asset naming convention
- docs consistency
- opt-equivalence (O0 == O1)

## Normalized Diff Basis

- **Baseline type**: worktree HEAD
- **Baseline reference**: `b53ebf271`
- **Comparison reference**: `recursive/718-stdlib-method-migration` (working tree)
- **Normalized baseline**: `git diff b53ebf271 -- <path>`
- **Normalized comparison**: `git diff b53ebf271..HEAD -- <path>`
- **Normalized diff command**: `git diff b53ebf271..HEAD -- std/ docs/ tests/`

## Router State

- `.recursive/config/recursive-router.json`: not present (no router configured for this run)
- `.recursive/config/recursive-router-discovered.json`: not present

## Subsequent Phases

All subsequent phases run from the worktree context:
`/home/wogikaze/arukellt/.worktrees/718-stdlib-method-migration`

## TODO

- [ ] Phase 1: AS-IS analysis — confirm inventory against current stdlib source in worktree
- [ ] Phase 2: Design — decide migration order, deprecation policy, per-type milestones
- [ ] Phase 3: Implementation — migrate free functions to trait/method syntax
- [ ] Phase 4: Verification — run verify quick, fix regressions
- [ ] Phase 5: Close — move issue to done/, commit
