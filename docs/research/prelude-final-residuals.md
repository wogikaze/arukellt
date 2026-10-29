# Prelude final wave: why `eq`, `clone`, `Box_new` / `unbox` still remain (2026-10-30)

This note records the measurements behind the four prelude frees that
`docs/adr/ADR-091-prelude-free-function-exception.md` §4 still defers after
`wave/prelude-text2`. It is evidence, not a decision: the decision stays in
ADR-091, and the numbers here describe a specific lane build, not a permanent
limit.

Lane: `.worktrees/wave-prelude-final` (`wave/prelude-final`, base `f0b1061ff`),
`ARUKELLT_BUILD_DIR=$PWD/.build-lane`, gc pin, warm stage-2 ≈ 50–90 s.

## `eq` — the pin cannot lower impl-based String equality

The recorded fix direction was: ship `std/core/cmp.ark` in the bootstrap flat
overlay (`BOOTSTRAP_STD_SOURCE_FILES` + the `std/core` copy list), then delete
the prelude free. The overlay change alone works — the pin builds a stage-2 that
compiles and runs `a.eq(b)` and `a == b` correctly for ordinary operands. The
retirement still does not land, for two measured reasons.

### 1. A globally visible `Ord` impl rewrites every comparison

Importing `std::core::cmp` from `std/prelude.ark` (needed so `impl Eq for
String` is in the module graph of every program) also registers `trait Ord` and
its impls. `typechecker::operator_traits::binary_trait_name` maps `<`, `>`,
`<=`, `>=` to `Ord`, so every comparison in every program starts dispatching
through `Ord::cmp`. The current lowering is not stable under that:

```text
tests/fixtures/stdlib_io_traits/file_read_eof.ark
  → error — compiler produced invalid core Wasm
    type mismatch: expected (ref null $type), found i32
```

Isolating the import shows it is the trigger:

| Prelude import | free `eq` | `file_read_eof.ark` |
|----------------|-----------|---------------------|
| `std::core::cmp` | removed | invalid core Wasm |
| `std::core::eq` (Eq only, new split) | removed | compiles |
| `std::core::cmp` | present (HEAD) | compiles |

Splitting `Eq` into `std/core/eq.ark` (`trait Eq` + its six impls, with
`std/core/cmp.ark` gaining `pub use std::core::eq` so `use std::core::cmp`
keeps providing `Eq`) removes that failure, but does not remove the next one.

### 2. `String ==` / `a.eq(b)` mis-stages operands whose MIR type is untracked

With only `Eq` visible, the remaining regression is the equality path itself.
`tests/fixtures/operators/string_eq.ark` (the ADR-005 content-equality fixture)
fails when an operand comes from an inline call:

```ark
fn accessors_match(values: Vec<String>, left: i32, right: i32) -> bool {
    values.get_unchecked(left) == values.get_unchecked(right)
}
```

- `literal == concat` (typed locals) compiles and runs.
- `words.get_unchecked(0) == words.get_unchecked(1)` → `error — compiler
  produced invalid core Wasm` (`type mismatch: expected (ref null $type),
  found i32`).

The free `eq(a, b)` never hit this: its declared `(String, String)` signature
staged both operands as refs, and `call_fallback_intrinsics` lowered the name to
the `text.string_eq` CoreOp. The `Eq` impl path has no such signature to stage
against; when the operand locals were not marked as `string`, the call lowers as
an i32 compare on two refs. Two attempts to repair it inside `mir_rewrite_call_target`
did not work and were reverted:

- rewriting the callee `String::eq` to `text.string_eq` → `expected (ref null
  $type) but nothing on stack` (the CoreOp handler expects a staged stack shape
  the method call does not have);
- marking both operands with `ctx_set_local_type(..., "string")` +
  `VT_GC_REF` before the call → unchanged failure.

**Prerequisite**: the bootstrap pin must lower `==`/`.eq()` on a `String`
receiver identically to the retired free *before* the compiler source can move
off the free. That is a pin-refresh item (the overlay packaging is already
sufficient); it is not an overlay omission.

### 3. The pin is not a valid oracle for `String ==` shapes

At clean HEAD the committed pin
(`bootstrap/arukellt-selfhost.wasm`) fails
`tests/fixtures/operators/string_eq.ark` with the same invalid-Wasm error,
while a freshly built stage-2 from the same source compiles it. Any bisection of
equality behaviour must use the lane `s2`, not the pin.

## `clone` — resolved in `wave/poly-clone` (2026-10-30)

The recorded E0200 failures reproduced at the base commit:

```text
error[E0200|typecheck]: type mismatch: String vs Vec
error[E0200|typecheck]: type mismatch: String vs struct
error[E0200|typecheck]: type mismatch: String vs enum
error[E0200|typecheck]: type mismatch: String vs ?
```

`clone(x)` is a typechecker special case keyed on the callee name
(`typechecker/call_fn.ark`), so it preserves the argument type. `.clone()` has no
equivalent: the method falls back to the bare prelude `clone`, which is declared
`(String) -> String`.

### Root cause of the `Vec<T>` invalid Wasm

A generic prelude declaration (`pub fn clone<T>(value: T) -> T`) makes `.clone()`
resolve for structs, enums, and `Option`, but `Vec<String>` then emitted invalid
core Wasm. Instrumenting `mir_rewrite_call_target` showed the receiver type is
correct (`argty=vec:String vt=5`); the callee was rewritten to `String::clone`
while `vec:i32` / `vec:Point` lowered as the identity `clone`:

```text
DBG-CLONE callee=clone argty=vec:String vt=5 resolved=String::clone
DBG-CLONE callee=clone argty=vec:i32 vt=5 resolved=clone
```

The emitted module confirms the type confusion:

```text
(func $String::clone (;17;) (type 33) (param $self (ref null 0)) (result (ref null 0)))
...
local.get $v            ;; (ref null 19) = Vec struct
call $String::clone     ;; expects (ref null 0) = String
```

`wasm-tools validate` reports
`func 18 failed to validate: type mismatch: expected (ref null $type), found
(ref null $type)`. The `vec:String` receiver spelling reaches the shared
callee/return cache as the bare `String` owner, so the Vec argument is passed
to the unrelated String impl method. Adding an inherent `Vec<T>::clone` gives
the bare-name rewrite a concrete owner and removes the hijack; the same fix
works for both targets.

### Mechanism that landed

1. `std/prelude.ark` keeps a **private** `fn clone<T>(value: T) -> T` as the
   polymorphic identity signature for receivers without a `Clone` impl. It is
   not importable and a bare call fails with E0100 plus a `.clone()` hint.
2. `std/prelude.ark` declares `impl<T> Vec<T> { fn clone(self) -> Vec<T> }`
   (shallow: the vector header is shared, element storage is not deep-copied).
   It is always in the module graph, so the compiler source (which does not
   import `std::collections::vec`) resolves `.clone()` on Vec the same way in
   the pin and in stage-2/3.
3. `std::collections::string::clone` was deleted. It reached
   `__core_string_clone_impl` through the retired CoreOp path and trapped with
   `wasm unreachable`; `s.clone()` is the replacement.
4. The resolver rejects `use std::prelude::{clone}` and reports the retired
   free with `removed_prelude_free_hint` / `removed_module_member_hint`.

No typechecker or MIR logic changed. That is deliberate: the committed pin
predates polymorphic method resolution, so the migrated source must compile
with the old pin. With only the std shim, the pin compiles `.clone()` on
`String`, `Vec<String>`, structs, enums, and `Option` on both `wasm32-gc` and
`wasm32`, and the stage-2 build of the fully migrated compiler succeeds.

### Residual

`Vec<T>::clone` is shallow (header shared) because a deep elementwise copy
needs `T: Clone` and `Vec<T>` carries values without Clone impls (compiler AST /
MIR records). That matches the pre-existing bootstrap-overlay lowering
(identity). A deep `impl<T: Clone> Clone for Vec<T>` can replace the inherent
method once the pin is refreshed and the source no longer needs the
`Clone`-impl-free path.

Lane evidence: 762 files / 13,984 call sites migrated; `verify fixtures`
1921/1921 PASS (38/38 `test:`), `verify lane` PASS, `docs check` 6/6, and the
ADR-029 fixpoint reached s2 == s3 at
`107c7c4d4bb7a072021c4277e20b0a1181f08603a863bbe384994de2a71b407e`
(pinned base `471855ad9db035c1ada136852fc55e896b477cf35303be066f352d9d563a8895`).

## `Box_new` / `unbox` — generic enum construction is not lowered

The resolver only accepts `Box::new` when `Box` is imported; otherwise the
error is `E0100 resolve: undefined module: Box`, not "undefined enum variant".
With `use std::prelude::{Box, unbox}` and a generic associated function:

```ark
impl<T> Box<T> {
    fn new(value: T) -> Box<T> { Box::Owned(value) }
}
```

`Box::new(...)` typechecks, then the compiler reports `error — compiler
produced invalid core Wasm` for both `Box::new(41)` and
`Box::new(String_from("hi"))`. Constructing the variant directly
(`Box::Owned(String_from("hi"))`) compiles but traps at runtime
(`unreachable`), so the generic enum payload lowering is broken before the
ownership checker's `Box_new` / `unbox` name special cases
(`typechecker/ownership.ark`) even enter the picture. Retiring the frees needs
that lowering fixed first; the name special cases cannot be replaced by
`.new()` / match today.

## Reproduction

```bash
export PATH=/home/wogikaze/.local/share/mise/installs/wasmtime/46.0.1/wasmtime-v46.0.1-x86_64-linux:$PATH
export ARUKELLT_CC=clang ARUKELLT_BUILD_DIR=$PWD/.build-lane
python3 scripts/manager.py selfhost build-compiler
export ARUKELLT_SELFHOST_WASM=$PWD/.build-lane/selfhost/arukellt-s2-runtime.wasm
scripts/run/arukellt-selfhost.sh compile --target wasm32-gc --emit component \
  -o /tmp/x.component.wasm tests/fixtures/operators/string_eq.ark
```

The full attempt (overlay + `std/core/eq.ark` + 5,728 migrated call sites +
diagnostics) was reverted rather than committed; the measurements above are the
reason. Lane gates with that attempt applied:
`verify fixtures` PASS=1912 FAIL=6, `arukellt test` 34/38, and no stub remained
in the built compiler after the five compiler-side receiver-typing sites were
given annotated locals.
