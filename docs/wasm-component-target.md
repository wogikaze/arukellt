# Component Model Target Evaluation

This note evaluates whether Arukellt should add a separate WebAssembly Component Model target instead of extending the current `wasm-js` and `wasm-wasi` targets.

## Short Answer

Yes, if Arukellt adopts the Component Model, it should be a separate experimental target track.

Do not mutate today's targets in place:

- keep `wasm-js` as the current embeddable core Wasm module ABI
- keep `wasm-wasi` as the current preview1 command-style ABI
- add a new target only when Arukellt is ready to commit to a Component-Model-specific host contract

The likely first shape is something like `wasm-component-js` or `wasm-component-preview2`, not a silent upgrade of `wasm-js`.

## Why A Separate Target

The current targets have intentionally narrow and concrete contracts:

- `wasm-js` exports source functions directly from a core Wasm module
- `wasm-wasi` exports `_start` and assumes preview1-style imports such as `fd_write` and `fd_read`

The Component Model changes the boundary itself:

- strings, lists, records, options, and results can cross the host boundary through canonical ABI lifting/lowering instead of Arukellt's current pointer-plus-memory conventions
- host imports are described as interfaces rather than raw imports on a core Wasm module
- adapter generation and runtime expectations change even when the source language stays the same

Because the ABI contract changes, overloading `wasm-js` or `wasm-wasi` would blur three materially different targets under the same name.

## Benefits

If Arukellt adds a Component Model target later, the upside is real:

- richer host boundaries without exposing raw linear-memory conventions
- better fit for `String`, `List<T>`, `Option<T>`, and `Result<T, E>` at the module boundary
- cleaner interop with host toolchains that already expect components and WIT-described interfaces

This is especially attractive if Arukellt grows toward "small tools with typed host calls" rather than only scalar core Wasm exports.

## Risks

This does not fit cleanly into the current prototype boundaries yet:

- Arukellt's backend still emits core WAT directly and is mid-stream on closure/runtime cleanup
- the current docs and tests assume either direct JS-host exports or preview1 command modules
- a component target would force new target docs, CLI semantics, fixture coverage, and host examples

There is also a product risk: introducing a component target too early would make users assume a broader host boundary is stable before the underlying core Wasm backend is settled.

## Recommended First Host Scenario

If this direction is pursued, the first concrete host scenario should be:

- a JavaScript or Node host that instantiates a component through component-aware tooling
- no browser promise at first
- no parity promise with `wasm-wasi`

Why this scenario:

- it avoids mixing Component Model rollout with WASI preview migration at the same time
- it gives Arukellt a typed host-boundary story without first redesigning the command-style `_start` target
- it keeps the experiment close to the current `wasm-js` mental model while still acknowledging a different ABI

## Recommended Contract

The safest initial contract is:

- new target name, separate from `wasm-js` and `wasm-wasi`
- explicit experimental label
- scalar-only guarantees can remain narrow at first
- typed lifting/lowering for strings and small aggregates may be added only when tested end to end
- no guarantee that existing `wasm-js` host glue continues to work unchanged

## Recommendation

Recommendation: viable, but not now as a production target.

Queue follow-up work only as an experimental track:

1. define the target name and CLI surface
2. document one host scenario and one non-goal scenario
3. add target-specific docs/tests before any backend implementation promise

Until then, keep Component Model work isolated from short-term backend size/perf slices.
