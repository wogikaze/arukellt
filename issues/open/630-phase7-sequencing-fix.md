---
Status: open
Created: 2026-05-16
Updated: 2026-05-16
ID: 630
Track: selfhost-retirement
Orchestration class: blocked-by-upstream
Depends on: 564, 575, 576, 577, 578, 579, 581
---

# Phase 7 Sequencing Fix: Circular Dependency in Crate Deletion Chain

## Discovery

During #574 (Delete `crates/ark-lexer`) pre-deletion verification, 3 active consumer crates
were found that still reference `ark_lexer::` symbols:

- `crates/ark-parser` (8 files) — references `Lexer`, `Token`, `TokenKind`, `tokenize`
- `crates/ark-resolve` (3 files) — references `ark_lexer::Token`
- `crates/ark-playground-wasm` — references `ark_lexer::TokenKind`

## The Problem

The current issue dependency chain is:

```
#564 (arukellt) → #574 (ark-lexer) → #575 (ark-parser) → #576 (ark-resolve) → #577 (ark-typecheck) → ...
```

But this is wrong. The crates have reverse dependencies:

- `ark-parser` (#575) depends on `ark-lexer` symbols
- `ark-resolve` (#576) depends on `ark-lexer` and `ark-parser` symbols
- `ark-playground-wasm` depends on `ark-lexer` symbols

So the deletion order should be:

1. First: migrate consumers to selfhost equivalents
2. Then: delete downstream crates before upstream crates (reverse of build order)

## Required Fix

The Phase 7 chain needs re-sequencing so that consumer crates are deleted first,
or a coordinated migration plan is executed:

1. Delete `crates/ark-playground-wasm` first (no downstream Rust consumers)
2. Delete `crates/ark-parser` (#575) — consumes ark-lexer
3. Delete `crates/ark-resolve` (#576) — consumes ark-lexer, ark-parser
4. Delete `crates/ark-typecheck` (#577), `crates/ark-hir` (#578), etc.
5. Finally delete `crates/ark-lexer` (#574) — last, since everything consumes it

Or alternatively: migrate the remaining references to use the selfhost equivalents
before starting any Phase 7 deletions.
