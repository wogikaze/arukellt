---
Status: open
Created: 2026-10-28
Updated: 2026-10-28
ID: 976
Track: language-design
Depends on: none
Orchestration class: design-ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 3
Source: "ABC481 arukellt port review (2026-10-28, HEAD 2436c92b0) + receiver-type env in ADR-053 plan"
Related: "#966, #933, #940, ADR-077, ADR-053"
---

# 976 — Collection mutators (`push`) are reported to resolve only on local-rooted receivers

## Summary

The ABC481 port review lists "`push` targets are limited to local variables" as a
language/implementation limitation (scored 6/10, "言語設計として検討"). The emitter
resolves `recv.m()` from a per-function local-type environment
(`docs/plans/selfhost-compiler-core-rewrite.md`, "receiver-type 環境": `let` / param /
`self` → local-type env), so a receiver whose type cannot be recovered from that
environment falls back to bare-name dispatch or an `unreachable` stub (#966/#940
families).

Observed surface on the current build (2026-10-28 16:45, s2 `ab08448d`):

| Receiver form | `wasm32` | `wasm32-gc` |
| --- | --- | --- |
| `v.push(x)` / `push(v, x)` on a local | runs | runs |
| `b.items.push(x)` (struct field of a local) | runs | runs |
| `g.get_unchecked(i).push(x)` / `push(g.get_unchecked(i), x)` | runs | runs |
| `make().items.push(x)` (call-result field) | compiles | not run |
| `self.items.push(x)` inside an impl method | runs | not run |
| `pair.0.push(x)` (tuple element) | parse error: expected `token` | — |

No failing `push`-specific receiver was found in this pass, so the recorded claim
needs a precise repro. The design question stands independently: should collection
mutators accept arbitrary place expressions, and how is the receiver type derived for
non-local places?

## Why it matters

- Collection methods are the main stdlib surface; arbitrary-place receivers
  (`vecs[i].push`, `s.items.push`, call-result fields) are normal code.
- If receiver typing is local-env-only, those forms either mis-dispatch by name
  (#966) or become stubs at runtime, which is a runtime trap rather than a
  typechecking error.

## Acceptance

- [ ] Minimal repro of a `push`/mutator call that is rejected or miscompiled because
      the receiver is not a local; record the exact place form.
- [ ] Decide the policy for method receivers on non-local places (receiver type
      inference, place evaluation, borrow/mutation rules) with an ADR reference
      (ADR-077 receiver resolution, ADR-053 core rewrite).
- [ ] If arbitrary places are supported by design: tests for tuple-element,
      nested-field and call-result receivers.
- [ ] If not: emit a diagnostic naming the unsupported place form instead of
      dispatching to a stub.

## Notes

- #966 covers same-named methods resolving to the wrong specialization;
  #933/#940 cover type identity and checker-to-lowering handoff.
- Tuple element access (`pair.0`) is a parser gap, not a push rule; a separate issue
  may be warranted if tuple element access is intended to be supported.
- Probe files: `tmp/issueprobe/push_field.ark`, `push_nested.ark`, `push_self.ark`,
  `push_generic.ark`, `push_temp2.ark`, `push_tuple.ark` (gitignored).
