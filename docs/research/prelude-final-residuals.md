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

The 2026-10-30 refresh attempt did not close that gap. The overlay packaging
was **not** sufficient: the closure omitted `std/core/scalar_format.ark` and
`std/core/raw.ark`, so `convert.ark`'s `__core_parse_f64_impl` call fell back
to the i32 `parse_f64` intrinsic and the gc self-build emitted invalid core
Wasm in `f64::try_from__String`. Shipping both modules makes the old pin's gc
self-build validate, but that pin is not faithful (p1 identity and gc
self-reproduction fail), and the faithful p1 s2 still hit a CoreOp
result/local typing bug (`try_emit_name_intrinsic_fallback`, func 5637).

That typing bug is now fixed: the `_to_string` suffix table in
`mir_core_call_string_result_name` was retyping the generated
`core_op_handler_text_to_string` accessor (an i32 handler id) as a `String`
ref, so `handler_id == core_op_handler_text_to_string()` lowered as reference
equality. The faithful p1 s2 now emits a gc core that validates
(`db883d23…`) and the packaged component reproduces it. The pin still is not
refreshed: p1 identity fails because the committed pin's build source is 39
compiler commits stale, which is a generation convergence problem, not this
typing bug. See
[pin-refresh-blocker-20261030.md](pin-refresh-blocker-20261030.md) for the
hashes and reproduction. The eq retirement therefore stays blocked on the
pin plus the eq-path operand staging described in §2.

**Update 2026-10-30 (`wave/pin-converge`)**: the pin oracle gap is closed. The
generation-converged pin `56ff0808…` compiles and runs
`tests/fixtures/operators/string_eq.ark` on both targets (32/32 lines), and the
ADR-029 fixpoint is `s2 == s3 == 61aa349b…`. The §2 operand-staging blocker is
also fixed by the source at this HEAD: an explicit `.eq()` chain with both
operands from inline `get_unchecked` calls (the exact `accessors_match` shape)
now compiles and runs on `wasm32` and `wasm32-gc` with the fixed s2.

The migration was then attempted in full on a scratch branch: split
`std/core/eq.ark` + `pub use std::core::cmp::Eq`, added the module to the
overlay closure, deleted the free from `std/prelude.ark` / `std/manifest.toml`,
added the `a.eq(b)` removal hint, and rewrote 5,729 free calls in 521 files
(`src/compiler`, `std`, `tests`) plus the selective imports. `selfhost
build-compiler` with the refreshed pin built an eq-source s2
(`71162e42…`) in 48 s, but that s2 **traps** inside `is_stdlib_path` while
compiling `string_eq.ark` on both targets. The producer pin never lowered
`.eq()` in its own source (the pre-eq tree had zero `.eq()` calls), and
`(path.slice(0, 5)).eq("std::")` is emitted as an unresolved-call trap stub;
the pre-eq free `eq(path.slice(0, 5), "std::")` worked because its declared
`(String, String)` signature staged the inline slice result. A bootstrap-only
free-`eq` shim appended to the overlay prelude did not change the result
(`16fbba74…` still traps), so this is receiver-type tracking for the method
call, not name resolution.

Consequence: the pin refresh alone does **not** unblock the retirement. It
needs either (a) `.eq()` receiver typing for inline `String::slice(...)`
results in the MIR call path (the §2 failure shape, one level down), or (b) a
producer pin already built from an eq-source compiler (two-generation
bootstrap). The attempt is preserved at
`.build-lane/pin-converge/eq-attempt.patch` (not committed); the codemod passes
`fmt --check` over all 453 compiler/std files, so the remaining work is the
receiver-typing fix, not the mechanical migration.

### Resolution 2026-10-30 (`wave/eq-retire`) — the parentheses were the trigger

The receiver-typing loss was not about `String::slice` typing or the overlay
closure. The migration codemod emitted `(receiver).eq(rhs)`, and a
parenthesized receiver is a distinct AST node, so
`lower_method_call_expr`'s composed-receiver check skipped the receiver
`LOCAL_GET`. The emitter's stack compose rescues that shape when the call is
a standalone expression, but not for the first expression after an `if`
statement: the `END` breaks the compose, and the call lowers to
`drop`/`unreachable` or `i32::eq`. A minimal probe shows it directly:

```ark
fn check(path: String) -> bool {
    if len(path) < 5 { return false }
    (path.slice(0, 5)).eq("std::")   // trap stub / i32::eq
}
```

`path.slice(0, 5).eq("std::")` (and `path.clone().eq(...)`,
`name.clone().eq(...)`) lowers correctly, as does the same parentheses form
before any `if`. So the unblock is to drop the unnecessary grouping
parentheses in the migration (5,422 receivers; only numeric literals keep
theirs). With that, the pin builds a non-trapping eq-source s2
(`12ba70e6…`), and `operators/string_eq` runs 32/32 on `wasm32-gc` and
`wasm32`.

Residuals: `std::collections::string::eq` stays public for the qualified-call
fixtures and is classified `must delete` (#917) in
`docs/data/stdlib-api-eradication.toml`; the underlying composed-receiver
bug in `lower_method_call_expr` / the emitter is still present for
parenthesized receivers after `if`, so the migration avoids the shape rather
than fixing it.

## `clone` — `.clone()` has no polymorphic resolution

`clone(x)` is a typechecker special case keyed on the callee name
(`typechecker/call_fn.ark`), so it preserves the argument type for
`Vec<String>`, `Vec<TypeInfo>`, `AstNode`, and every other value without a
`Clone` impl. `.clone()` has no equivalent: the method falls back to the bare
function `clone`, which is declared `(String) -> String`, so non-String
receivers fail typechecking:

```text
error[E0200|typecheck]: type mismatch: String vs Vec
error[E0200|typecheck]: type mismatch: String vs struct
```

A generic prelude declaration (`pub fn clone<T>(value: T) -> T`) makes
`.clone()` resolve for structs, enums, and `Option`, but `Vec<T>::clone` then
emits invalid core Wasm, so the 13,947 free call sites in 743 compiler / std
files cannot be migrated yet. `std::collections::string::clone` is also a dead
path: its body reaches `raw_string_clone`, a retired CoreOp
(`resolver::removed_names`), and traps when called.

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

### Resolution (2026-10-30, `wave/box-new`)

The blocker was the mono method return signature, not the payload lowering
alone. `ctx_maybe_enum_return_type_name` collapsed the constructor's declared
`Wrap<i32>` return to `enum:Wrap`, and
`resolve_mono_method_return_value_sig` read the receiver-shaped name as a
generic method returning its type parameter, substituting the mono key (`i32`
or `String`) as the Wasm result type. Return positions now keep
`enum:Wrap<i32>`, `let`/call-result locals keep the instantiation arguments,
and a generic ref payload bind replaces its `enum:<T>` placeholder once the
scrutinee supplies arguments. `impl<T> Box<T> { new, take }` lives in
`std/prelude.ark`; `Box_new` / `unbox` are retired with E0100 hints and the
borrowed-`Box` E0227 sink now recognizes `Box::new` / `.take()`. Evidence:
`verify fixtures` 1926/1926, `verify lane`, `docs check` 6/6, and the ADR-029
fixpoint (`ddf46a2f…`).

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
