---
Status: open
Created: 2026-05-14
Updated: 2026-05-14
ID: 618
Track: component-model
Depends on: 262, 476
Orchestration class: design-ready
Orchestration upstream: None
---

# WIT bindings round-trip regression surface

## Summary

#262 closed the component-interop regression surface, but its future WIT
round-trip bullet covered a different workflow: generate WIT from Arukellt,
generate bindings back from that WIT, and verify those bindings execute through
the component pipeline. This issue tracks that workflow as its own open slice so
#262 can remain complete without hiding the bindings-generation gap.

## Why this matters

- Component interop smoke tests can pass while generated WIT is not usable as an
  input to binding generation.
- `wasm-tools compose` coverage (#476) verifies composition, not Arukellt
  binding regeneration from emitted WIT.
- The generated WIT contract should be executable evidence, not only a text
  artifact.

## Acceptance

- [ ] A fixture emits WIT from Arukellt source and stores the expected WIT shape.
- [ ] A bindings-generation step consumes the emitted WIT and produces Arukellt
  bindings or an explicitly documented interim binding artifact.
- [ ] The generated bindings participate in a round-trip smoke test through the
  component pipeline.
- [ ] The workflow is wired into `tests/component-interop/` or an adjacent
  component test directory with a stable runner.
- [ ] `python scripts/manager.py verify` passes.

## Primary paths

- `tests/component-interop/`
- `crates/arukellt/src/`
- `crates/ark-wasm/src/`
- `docs/testing/test-categories.md`

## Close gate

All acceptance items checked with repo-internal evidence; #262 does not regain
unchecked future bullets.
