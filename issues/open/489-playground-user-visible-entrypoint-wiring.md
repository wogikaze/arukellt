# Playground user-visible entrypoint wiring

**Status**: open
**Created**: 2026-04-13
**Updated**: 2026-04-13
**ID**: 489
**Depends on**: 382, 437, 438, 464
**Track**: playground
**Blocks v1 exit**: no
**Priority**: 40

## Created by audit — 2026-04-13

**Source**: Audit found that `playground/src/` contains components for examples, share, highlight, and capability checks, but `docs/playground/index.html` does not wire them into a user-visible experience. No open issue tracked this integration gap.

## Summary

The playground directory contains TypeScript modules implementing individual features (example loading, share URL, syntax highlight, runtime capability check), and `crates/ark-playground-wasm/` exposes a WASM API. However, the user-visible HTML entrypoint at `docs/playground/index.html` does not import or initialise these modules, so users cannot access them.

## Acceptance

- [ ] `docs/playground/index.html` imports and initialises the playground WASM module
- [ ] Example selector is rendered and functional
- [ ] Share link generation works end-to-end in a browser
- [ ] Syntax highlighting is applied to the editor area
- [ ] Capability check warns when required browser APIs are absent
- [ ] Build step produces the wired bundle (`playground/` build succeeds)

## Primary paths

- `docs/playground/index.html`
- `playground/src/`
- `crates/ark-playground-wasm/`

## Required verification

- `cd playground && npm run build` succeeds
- Manual or automated browser check confirms editor loads

## Close gate

- User-visible playground page loads WASM, shows editor, can run examples, and can share links
