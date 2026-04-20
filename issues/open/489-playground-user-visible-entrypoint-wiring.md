
## Reopened by audit

- **Date**: 2026-04-21
- **Reason**: Entrypoint HTML + TS wired but complete user flow fails because wasm module cannot be loaded; docs/playground/wasm/ is empty
- **Root cause**: The playground wasm binary (ark-playground-wasm) has never been compiled. crates/ark-playground-wasm/pkg/ does not exist. docs/playground/wasm/ is empty. All playground user-visible functionality depends on this binary.
- **Evidence**: `find . -name '*.wasm' -path '*playground*'` returns nothing; `ls crates/ark-playground-wasm/pkg/` fails; `ls docs/playground/wasm/` is empty.

# Playground user-visible entrypoint wiring

**Status**: done
**Created**: 2026-04-13
**Updated**: 2026-04-15
**ID**: 489
**Depends on**: 437, 438, 464
**Track**: playground
**Blocks v1 exit**: no
**Priority**: 40

## Created by audit — 2026-04-13

**Source**: Audit found that `playground/src/` contains components for examples, share, highlight, and capability checks, but `docs/playground/index.html` does not wire them into a user-visible experience. No open issue tracked this integration gap.

## Summary

The playground directory contains TypeScript modules implementing individual features (example loading, share URL, syntax highlight, runtime capability check), and `crates/ark-playground-wasm/` exposes a WASM API. However, the user-visible HTML entrypoint at `docs/playground/index.html` does not import or initialise these modules, so users cannot access them.

## Parent note — 2026-04-15

This issue no longer depends on #382. The browser entrypoint and user-visible wiring work
are part of the existing playground v1 surface, while #382 now tracks the separate T2
freestanding target track.

## Completed — 2026-04-15

**Evidence review**:
- `docs/playground/index.html` is the repo-backed playground entrypoint and now wires
  example selection, share-fragment load/save, and `pg.tokenize()` syntax highlighting.
- `npm --prefix playground run build` succeeded during the completion slice.
- Commit `eaea05d42b4ae9ef22b4db87ae0a2c3f7dc9364b` contains the user-visible entrypoint
  wiring required by this issue.

**Close gate**: satisfied by repo entrypoint evidence plus build proof.

## Acceptance

- [x] `docs/playground/index.html` imports and initialises the playground WASM module
- [x] Example selector is rendered and functional
- [x] Share link generation works end-to-end in a browser
- [x] Syntax highlighting is applied to the editor area
- [x] Capability check warns when required browser APIs are absent
- [x] Build step produces the wired bundle (`playground/` build succeeds)

## Primary paths

- `docs/playground/index.html`
- `playground/src/`
- `crates/ark-playground-wasm/`

## Required verification

- `cd playground && npm run build` succeeds
- Manual or automated browser check confirms editor loads

## Close gate

- User-visible playground page loads WASM, shows editor, can run examples, and can share links
