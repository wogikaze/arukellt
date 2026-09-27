---
Status: done
Created: 2026-09-27
Updated: 2026-09-27
ID: 890
Parent: 852
Track: language-v2
Depends on: "860"
Related: "ADR-058, RFC-011, #852, #860, #888"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 2
Source: "#888 receipt — Option<(K,V)> payload shape breaks on both backends"
---

# 890 — Option<tuple> payload produces invalid wasm on both backends

## Scope

`Option<(A, B)>` — an Option whose payload is a tuple — lowers to wasm
that fails validation on both `wasm32` (linear memory) and `wasm32-gc`.
Found while probing `HashMap::entries()` consumers for #888:
`map.entries().get(0)` / `map.entries().pop()` return
`Option<(K, V)>` and hit the same shape.

Minimal reproducer (probes in `.build/probe/`, this branch):

```ark
use std::host::stdio
fn mk() -> Option<(i32, i32)> {
    Some((7, 8))
}
fn main() {
    let r = mk()
    match r {
        Some(p) => { let (a, b) = p
                     stdio::println((a + b).to_string()) }
        None => stdio::println("none")
    }
}
```

- `wasm32` (LM): wasmtime reports
  `function[10]::main ... type mismatch: values remaining on stack at
  end of block` — the Some payload unpack/destructure path leaves a
  ref value unaccounted for in the block type.
- `wasm32-gc`: core module fails `wasm-tools validate` the same way
  (`func main`, values remaining on stack at end of block).

Plain `Vec<(i32, i32)>` with `get_unchecked` + `let (a, b) = e`
destructure works on both backends (#888 verified), so the gap is
specific to the Option-with-tuple-payload ABI — likely in the
enum-payload layout / `MIR_TUPLE_LIT` payload packing or the
`Some(p)` pattern-bind destructuring path, not in tuple handling
generally.

## Acceptance

- [x] `Option<(i32, i32)>` (and at least one `(String, i32)` pair)
      compiles, validates, and runs identically on wasm32-gc and wasm32
- [x] `Vec<(K,V)>.get(i)` / `.pop()` on HashMap entries results work
      through `match Some((k, v))` and `let (k, v) = p`
- [x] positive fixture on both backends

## Required evidence

- implementation commit SHA
- positive fixture + applicable compile/validate/runtime gate output

## Progress receipt — 2026-09-27 (issue closing)

Commit: `62048aded` on `docs/adr-058-language-v2-semantics`.

Root cause chain: `mir_type_info_to_mono_key` returned an empty mono
key for non-empty tuples (only `()` mapped to `unit`), so
`Option<(A, B)>` registered its return type as bare `option:`. Every
downstream consumer then saw an untyped Option: `Some(p)` bound `p`
as `enum:open`, which on wasm32-gc is a generic ref that fails
field-index bounds, while on wasm32 the destructure defaulted element 1+
to GC ref and the scalar arithmetic collapsed into a dropped string
conversion (operand-stack leak / unset local read). The stage-2
bootstrap stub in `scripts/selfhost/checks.py` shadows
`flat-src/mir_lower_ctx_mono_type_params.ark`, so the same tuple
branch was mirrored there and the flat-overlay disk cache bumped to v4.

Three more gaps surfaced once the payload name flowed through:

- `mir_emit_tuple_from_locals` only set the local type when the named
  GC-struct path ran, so LM small-int tuples (`let t = (1, 2)`) left
  the local untyped and destructure mistyped later fields — it now
  records `tuple:<k,...>` on the anonymous path too.
- `lower_tuple_destructure_let` lowered a bare-identifier init with
  `lower_expr`, emitting a `local.get` no instruction consumed — the
  pushed copy leaked onto the operand stack (block-end mismatch inside
  match arms, frame-leave corruption at top level). It now resolves via
  `ctx_lookup_local`.
- `Some((k, v))` bound the whole tuple pattern as one flat local
  under the pattern node's text, so `k`/`v` never existed (stack
  underflow). Tuple sub-patterns now bind an unnamed temp for the
  payload and destructure each element through the shared
  `emit_tuple_pattern_ident_binds` extracted from `stmt_let_tuple`.

Evidence:

- Implementation commit `62048aded`
  (`src/compiler/mir/lower/ctx_mono_type_params.ark`,
  `aggregate_tuple.ark`, `match_payload_fields.ark`,
  `core_match_payload_bind_core.ark`, `stmt_let_tuple.ark`,
  `scripts/selfhost/checks.py`).
- Positive fixture `tests/fixtures/enums/option_tuple_payload.ark`
  covers `Option<(i32, i32)>` and `Option<(String, i32)>` Some
  binds, `Some((k, v))` on HashMap `entries().get(0)`/`pop()`,
  `let (k, v) = p` destructure, and a bare tuple let — registered
  `run:` + `t3-compile:` + `t3-run:`. Output
  `15 hi42 k17 k17 12` is identical on the wasm32-gc component and
  the wasm32 core module; `wasm-tools validate` clean on both.
- Probe matrix re-verified on the final s2: `opt_tuple`,
  `opt_tuple_str`, `opt_tuple_annot`, `let_destructure`,
  `hm_entries` all compile/validate/run with identical output on
  both backends.
- `python3 scripts/manager.py selfhost build-compiler`: PASS 1/1 on
  the final source.
- `python3 scripts/manager.py verify lane`: EXIT=0 (comment policy,
  semantic-debt, fmt, lint smoke, lint ratchet W0011 back to baseline,
  touched-code quality ratchet, quality contract).
