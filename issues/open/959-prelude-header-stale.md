---
Status: open
Created: 2026-10-28
Updated: 2026-10-28
ID: 959
Track: docs-audit
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 3
Source: "ABC481 arukellt port + WAT emission session (2026-10-28, HEAD 0a64bdb62)"
Related: "ADR-072, #878, #868"
---

# 959 — `std/prelude.ark` header still says 'auto-imported into every module', contradicting the ADR-072 v2 policy

## Summary

The file header of `std/prelude.ark` still claims:

```
// Arukellt Standard Prelude — auto-imported into every Arukellt module
...
// ## Tiny Prelude ... Functions provided by THIS file ...
```

After ADR-072 / #878 only the tiny stable set is auto-imported
(`std/manifest.toml [prelude_policy]`, `src/compiler/resolver/prelude_stable.ark`);
every other prelude declaration requires `use std::prelude::{name}` and bare calls fail
with `E0100` plus a migration hint. `docs/stdlib/prelude.md` already describes this
split correctly, so only the source comment is stale.

This misleads readers (and agents) into expecting `to_string`, `join`, `byte_len`,
`args`, ... to be in scope; they are not.

## Fix

- Rewrite the header comment to state the v2 contract and link ADR-072 /
  `docs/stdlib/prelude.md` / `std/manifest.toml`.
- Sweep other sources for the same stale wording (`grep -rn "auto-imported" std src docs`).

## Acceptance

- [ ] `std/prelude.ark` header matches ADR-072 and `docs/stdlib/prelude.md`.
- [ ] No remaining "auto-imported" claim that contradicts `[prelude_policy]`.
- [ ] No behavior change (docs-only diff).

## Update (2026-10-28, Wave A)

The `std/prelude.ark` header now states the v2 contract: only the
`std/manifest.toml [prelude_policy]` stable set is auto-imported, and every
other prelude declaration resolves only through `use std::prelude::{name}` (or
its canonical replacement module); bare calls fail with E0100 plus the
`resolver::removed_names` hint. It points at ADR-072,
`docs/stdlib/prelude.md`, `src/compiler/resolver/prelude_stable.ark`, and
`scripts/check/check-compat-orphans.py`. The stale "canonical v3+ — ≤ 15
symbols" wording and the `Bytes` entry (not in `stable_types`) were removed.
Comment-only change; no behavior change.

Sweep (`grep -rni 'auto-import' std src docs`): claims that still contradict
the v2 policy and sit outside this issue's write scope:

- `src/compiler/resolver/builtins_core_collections.ark:1`
- `src/compiler/resolver/builtins_core_prelude.ark:1`
- `src/compiler/resolver/builtins_core_result_math.ark:1`
- `src/compiler/resolver/builtins_convert.ark:1`
- `src/compiler/resolver/builtins_hof.ark:1`
- `src/compiler/resolver/builtins_core_string_tail.ark:1`

They all say the functions are "now auto-imported from std/prelude.ark"; the
declarations live there but need an explicit `use` outside the stable set.

- `docs/stdlib/module-system.md:80,94` — "The prelude (`std/prelude.ark`) is
  always auto-imported."
- `docs/stdlib/migration-guidance.md:1678` — "The auto-imported prelude
  `args()` binding"; `args` is not in the stable set.
- `std/manifest.toml:58,144` — section comments "Prelude types (auto-imported)"
  / "Prelude values (auto-imported)" sit above tables that also contain legacy
  entries; ambiguous rather than false.

`docs/stdlib/prelude-dedup.md` and `prelude-migration-inventory.md` describe the
pre-#878 legacy boundary and were left as history.

Verification: `python3 scripts/manager.py fmt std/prelude.ark` PASS,
`python3 scripts/check/check-comment-policy.py --changed` PASS,
`python3 scripts/manager.py docs check` 6/6 PASS.

## Implementation (2026-10-28, Wave A second pass — full sweep)

All sweep items listed in the previous update are now fixed (comment/doc text
only; no code or resolver behavior change):

- `src/compiler/resolver/builtins_core_collections.ark`,
  `builtins_core_prelude.ark`, `builtins_core_result_math.ark`,
  `builtins_convert.ark`, `builtins_hof.ark`,
  `builtins_core_string_tail.ark`: headers now say the functions are declared in
  `std/prelude.ark` (or registered in `builtins_intrinsics.ark`) and need an
  explicit `use` outside the v2 stable set (ADR-072). The `builtins_convert.ark`
  note also records that `__intrinsic_stdin_read_to_string`'s public wrapper is
  `std::host::stdio::read_stdin`.
- `docs/stdlib/module-system.md`: "Prelude interaction" now states the v2 stable
  set and the E0100/explicit-`use` rule for other prelude declarations; the
  directory-convention note says the v2 stable set is auto-imported and other
  decls need `use`.
- `docs/stdlib/migration-guidance.md`: the env-wave note now says
  `std::prelude::args` is outside the auto-imported stable set and needs
  `use std::prelude::args`.
- `docs/stdlib/prelude-dedup.md`: "Legacy Prelude functions" are no longer
  "available without import"; the resolver order notes that only the stable set
  is auto-imported.
- `docs/stdlib/generation-schema.md`: the `prelude` field no longer claims it
  means "auto-imported"; it marks prelude-surface membership, with auto-import
  limited to `[prelude_policy]`.
- `std/manifest.toml:58,144`: section headings now read
  "Prelude types/values (stable set auto-imported; other entries
  explicit-import only)".

Both docs files edited here are hand-written (`docs/directory-ownership.md`
lists only `docs/stdlib/reference.md`, `modules/*.md`, `scoreboard.md`, and
`eradication.md` as generated), so no regeneration is required.

Remaining `auto-import` mentions that are accurate or historical and were left
alone:

- `docs/adr/ADR-072-…md:31,68` — accepted decision record describing the
  migration period; not rewritten here (ADR progress wording is a separate
  decision).
- `docs/rfcs/005-…md`, `docs/rfcs/011-…md` — design RFC text.
- `docs/language/spec.md:1603,1606,1913` — already describes the v2 split
  correctly.
- `docs/stdlib/prelude.md:9,49` — already correct.
- `docs/stdlib/prelude-migration-inventory.md` / `docs/stdlib/README.md:132` —
  #513 working note (historical shelf list).
- `docs/tooling-feature-matrix.md:42` — LSP "auto-import" code action, unrelated.
- `src/compiler/resolver/prelude_stable.ark`, `register_imported.ark`,
  `removed_names.ark`, `builtins_intrinsics.ark` — correct descriptions of the
  auto-import set / removal hints.
- `std/manifest.toml:33,35,988` — correct v2 policy / legacy-boundary notes.

Verification: `python3 scripts/manager.py fmt --check <6 resolver .ark>` PASS
(checked=6 failed=0); `python3 scripts/check/check-comment-policy.py --changed`
PASS; `python3 scripts/manager.py docs check` 6/6 PASS (docs freshness included).
No generated file was hand-edited and no regeneration was needed.
