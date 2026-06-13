---
Status: done
Created: 2026-06-12
Updated: 2026-06-12
ID: 635
Track: playground
Depends on: none
Orchestration class: implementation-ready
Blocks v1 exit: none
Source: docs-to-issues audit — docs/process/docs-gap-inventory-2026-06-12.md
---

# 635 — Playground: wire format and tokenize in browser UI

## Summary

The TypeScript playground engine exports `formatSource` and `tokenizeSource`, but the browser entrypoint at `docs/playground/index.html` does not expose format or tokenize actions. Docs explicitly mark this as not yet wired.

## Evidence source

docs/playground/README.md (L36-48), docs/language/README.md (L67), playground/src/engine.ts

## Primary paths

playground/src/, docs/playground/index.html, docs/playground/ (generated README source)

## Non-goals

T2 execution, compiler-wasm run loop (#632 done), Lighthouse CI (#498)

## Acceptance

- [x] Browser UI exposes format action that calls engine format API and updates editor content
- [x] Browser UI exposes tokenize action or uses tokenize for syntax highlighting beyond parse-only path
- [x] docs/playground/README.md format/tokenize rows updated to repo-proved (regenerate via generate-docs.py if needed)
- [x] docs/playground/README.md typecheck tracking row updated (#472 is done; remove stale issues/open/472 reference)
- [x] playground npm build succeeds and pages workflow deploy path remains green

## Required verification

```bash
cd playground && npm run build:app
python3 scripts/manager.py verify quick
```

## Close gate

Manual browser smoke: open docs/playground/index.html, run format and tokenize actions on sample source; verify README snapshot rows match repo proof.
