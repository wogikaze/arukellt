---
Status: open
Created: 2026-10-19
Updated: 2026-10-19
ID: 931
Track: compiler
Depends on: none
Orchestration class: research-needed
Orchestration upstream: none
Blocks v{N}: none
Priority: 2
Source: "#929 verification — module-graph compile of
  src/compiler/main.ark now loads all modules and stops at
  cross-module visibility errors"
Related: "#927, #929, #851"
---

# 931 — module-graph compile: sibling files access each other's
# private items; nested tree needs a visibility model

## Summary

With the #929 root-resolution fix, `compile src/compiler/main.ark`
through the module-graph path loads every module and reaches name
resolution, where it reports cross-module visibility errors:

    error[E0100|resolve]: visibility error: `cli_input_file` is
      private and cannot be accessed from module `args_record`
      --> src/compiler/main/editor.ark

`src/compiler/main/args_record.ark` defines 86 fns, none `pub`;
sibling files (`args_parse.ark`, `compile_all.ark`,
`compile_core.ark`, `component_cmd.ark`) call them as
`args_record::cli_*` after `use main::args_record`. Under the flat-src
overlay the namespace merge makes these accesses intra-module, so the
canonical path compiles; under real module resolution each file is a
separate module and the accesses are genuine privacy violations.

## Design question

This is a language-semantics fork, not a bug with one obvious patch:

- **Strict file-as-module** (current nested resolution): sibling
  files must `pub` what they share. `src/compiler/**` would need
  hundreds of `pub` additions to compile via module graph — the
  overlay exists precisely because the tree was not written that
  way. User-facing fixture packages (`tests/fixtures/modules/*`)
  already follow this rule and declare `pub fn`.
- **Directory-as-namespace**: files in one directory share a
  namespace so private stays file-scoped *within* the same dir
  module family. Matches how the selfhost tree is actually written,
  but weakens the meaning of a nested `use` and needs an ADR-level
  decision on where module boundaries sit.

Either way the flat-src path stays canonical for selfhost builds;
this only affects direct whole-tree compiles and nested user
packages that rely on same-dir private sharing (which today fail
the same way under the pinned compiler — pre-existing semantics).

## Evidence (2026-10-19)

- `lin_comp_v29` (fixed-GC + #929 loader): `compile
  src/compiler/main/editor.ark --target wasm32-gc` reports repeated
  E0100 visibility errors for `cli_input_file` /
  `cli_has_input_file` — zero "failed to load module" errors.
- `tests/fixtures/modules/*` precedent: cross-module access uses
  `pub fn` (e.g. `use_local_module/helpers.ark`).

## Acceptance

- [ ] ADR or spec note decides file-as-module vs
      directory-namespace for sibling private access.
- [ ] `compile src/compiler/main.ark` through module graph either
      completes or documents the strict-module requirement with a
      tracked mechanical `pub` pass.
