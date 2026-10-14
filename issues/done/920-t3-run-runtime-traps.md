---
Status: done
Created: 2026-10-12
Updated: 2026-10-14
ID: 920
Track: wasm-feature
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 2
Source: #721 close verification — full t3-run execution sweep (2026-10-12)
---

# t3-run fixtures trap at runtime on wasm32-gc (pre-existing codegen bugs)

## Summary

During the #721 full-execution sweep of all 489 `t3-run:` manifest fixtures
under wasmtime 46 (compile via current selfhost → wasm-tools validate → P2
component package → wasmtime run → `.expected` compare), two fixtures trap
with a wasm `unreachable` at runtime. Both traps reproduce identically when
the emitted module's `(sub final ...)` entries are rewritten to open
`(sub ...)` (wasm-level A/B), so they are **not** caused by final types and
predate the #721 verification run.

## Failing fixtures

### 1. `modules/multi_import/main.ark`

Prints `30` (expected `30\nababab`), then traps:

```
wasm trap: wasm `unreachable` instruction executed
  0:   0x1140 - <unknown>!main
  1:   0x11c5 - <unknown>!<wasm function 19>
```

The trap happens after the first `stdio::println`, inside the call to the
module-imported `strings::repeat(String_from("ab"), 3)`. Suspect: return
value ABI or callee lowering for module-imported functions returning
`String` on wasm32-gc.

### 2. `stdlib_trait/ord_sort_vec_string.ark`

Traps inside the monomorphized `seq__sort_by__Reverse_i32_` (Ord-based sort
of `Vec<cmp::Reverse<i32>>`, #695):

```
wasm trap: wasm `unreachable` instruction executed
  0:   0x1775 - <unknown>!seq__sort_by__Reverse_i32_
  1:   0x2447 - <unknown>!main
  2:   0x2522 - <unknown>!<wasm function 28>
```

The Vec<String> half of the fixture (first `seq::sort_by` + asserts) does not
print; the trap site is the Reverse<i32> instantiation — suspect a generic /
Ord-dispatch stub left as `unreachable` in the monomorphized body.

## Reproduction

Canonical parity pipeline per fixture:

```
compile <fixture> --target wasm32-gc --wasi-version wasi-p2 --memory-width memory32
wasm-tools component embed scripts/selfhost/wit/deps/wasi-cli-0.2.0/wit --world command core.wasm
wasm-tools component new embedded.wasm --reject-legacy-names --realloc-via-memory-grow
wasmtime run --wasm gc --wasm function-references --dir <root> component.wasm
```

Evidence receipt: `docs/research/receipts/721-t3-run-execution-20261012.json`.

## Acceptance criteria

- [x] `modules/multi_import/main.ark` runs to completion and prints
      `30\nababab` under wasmtime 46.
- [x] `stdlib_trait/ord_sort_vec_string.ark` runs to completion (exit 0)
      under wasmtime 46.
- [x] Root cause identified and fixed (or an accepted SKIP with linked
      follow-up, per false-done prevention rules).

## Notes

- The two `stdlib_hashmap` t3-run fixtures that timed out in the same sweep
  are **not** defects: they compile in ~0.7s under the gc selfhost host and
  pass end-to-end; the timeouts are the linear-host mark_candidate compile
  latency tracked by #851.

## Resolution (2026-10-14, e7e745e9c)

Fixed in two parts.

multi_import: deferred stdlib decls (CoreOp shim / intrinsic body rewrite)
registered signatures that never produce a MirFunction and occupied the
same-name fid-chain slot, so the emitted strings::repeat bound the deferred
decl's FunctionId and its own fid stayed unmapped -> unpatched call 0.
SignatureRegistry now tracks a signature_only flag (set via
module_fn_is_stdlib + mir_defer_body_for_decl plumbing through the loader /
corehir / driver chain), and ctx_fid_map_build_registry_chains plus
mir_reachability_registry_id_for_ordinal skip signature-only entries.

ord_sort_vec_string: enclosing template sites never rebound for
module-qualified mints (sort_by vs seq::sort_by), leaving Reverse::cmp
unminted -> drop;unreachable in seq__sort_by__Reverse_i32_.
rebind_enclosing_template_sites now matches exact names, impl-owner
type-arg-erased names (Reverse<I>::cmp vs Reverse::cmp), or last-segment
names only for bare enclosing decls. A purely unqualified compare was tried
first and rejected: it collapses every Owner::cmp/eq/next impl into one
bucket and the mint closure explodes into an emit-phase stall against the
pinned-bootstrap linear GC's O(roots x allocs) collect.

Latent fix in the same commit: main/compile_all.ark called the renamed-away
print_compile_errors -> print_compile_diagnostics.

Verified: both fixtures compile (wasm32-gc) and run clean via component
pipeline under wasmtime 46 (multi_import prints "30\nababab";
ord_sort_vec_string exits 0, all asserts). python3 scripts/manager.py
verify lane PASS.
