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

**Mandatory:** [ADR-046](docs/adr/ADR-046-free-function-eradication.md) (ACCEPTED).
User-reachable free functions are **eradicated**. Do not treat free functions,
prelude wrappers, or “temporary bridges” as a lasting public API
(also issue #709).

The stdlib public surface is **trait-first / type-first** (ADR-044, ADR-036,
ADR-046):

1. **Shared behavior → `trait` + `impl Trait for Type`**
   Prefer a trait when the operation is reusable across types (scalars,
   collections, etc.). Example — **correct**:

   ```ark
   trait Integer {
       fn is_power_of_two(self) -> bool
   }
   impl Integer for i32 {
       fn is_power_of_two(self) -> bool {
           self > 0 && (self & (self - 1)) == 0
       }
   }
   ```

   **Incorrect end state:** only `impl i32 { fn is_power_of_two(...) }` with
   no trait, when the same op belongs on `i64` / other integers too.
   Inherent `impl Type` alone is a stopgap only when the behavior is
   truly type-unique; default to a trait for cross-type APIs.

2. **Require method / associated call sites**
   `s.split(sep)`, `v.push(x)`, `n.to_string()`, `n.is_power_of_two()` —
   never lasting `split(s, sep)`, `push(v, x)`, `i32_to_string(n)`,
   `is_power_of_two(n)`.

3. **Associated constructors / namespace ops**
   `Vec::new()`, `String::from("x")`, `Env::args()`, `Process::exit(c)` —
   not `Vec_new_i32()`, not free `args()` / `exit()`.

4. **Deprecation only, then delete**
   Public / prelude thin wrappers are migration-only (ADR-014 + W0009).
   Final form is trait method, inherent method (type-unique only), or
   associated function — never a free function.

5. **Exceptions (only)**
   Non-public `__intrinsic_*` / manifest `kind = "intrinsic"`.
   Private free helpers inside `std` are also forbidden in principle.

6. **Monomorphic helpers** (`*_i32`, `*_i64`, `*_f64`) are delete targets
   (#703), not user-facing API.

**Agent anti-pattern (do not repeat):** migrating `math::is_power_of_two(n)`
by adding only `impl i32 { fn is_power_of_two }` and calling Tier 1 “done”.
That satisfies method syntax but **violates trait-first**. Introduce the
shared trait (e.g. `Integer`) and `impl` for each integer type.

References: **ADR-046** (free-function eradication — required reading),
ADR-044 (trait/method syntax), ADR-036 (trait-stdlib-redesign; D5 withdrawn),
ADR-038 (operator overload traits), issue #709, issue #718, issue #703.

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
