# Core Ops Registry

> Registry path: [`data/core-ops.toml`](../../data/core-ops.toml)
> Status: designated future SSOT; currently `status = "scaffold"` and not consumed by the compiler
> Proposed design: [ADR-042](../adr/ADR-042-intrinsic-layer-separation.md) D5
> Migration owner: [issue #798](../../issues/open/798-adr-042-semantic-operation-registry-migration.md)

## Role

`data/core-ops.toml` is the **designated future single source of truth** for
semantic types, `CoreOpId`, visibility, classification, binding policy, effect,
inline policy, lowering, fallback, and differential equivalence.

`status = "scaffold"` means the compiler does not consume it as an authoritative
input today. Issue #798 owns adoption after ADR-042 acceptance.

| Concern | Owner |
|---------|-------|
| Current semantic registration and lowering | resolver / typechecker / MIR / emitter code |
| Public path, docs, stability, deprecation | `std/manifest.toml` |
| Future semantic metadata | `data/core-ops.toml` (ADR-042) |

`std/manifest.toml` entries may reference `core-ops.toml` via `core_op_id`
(for `[[functions]]`) and `type_id` (for `[[types]]`). Example:

```toml
# std/manifest.toml (illustrative)
[[types]]
name = "String"
type_id = "string"
stability = "stable"

[[functions]]
name = "starts_with"
module = "std::text"
core_op_id = "string.starts_with"
stability = "stable"

[[functions]]
name = "starts_with"
kind = "prelude_wrapper"
core_op_id = "string.starts_with"
stability = "stable"
```

The `core_op_id` / `type_id` references shown above are syntactically accepted
and used by the example entries, but `core-ops.toml` is not yet consumed by the
compiler. Issue #798 owns adoption after ADR-042 acceptance.

A single `CoreOpId` may be referenced by multiple public bindings (e.g.
`prelude::starts_with` and `std::text::starts_with`). All public bindings of
the same `CoreOpId` must be consistent with the canonical `signature` in
`core-ops.toml`.

## Registry schema

The current schema is `schema_version = 4`. See `data/core-ops.toml` for the
reference entries. Each operation has:

- `id` — canonical `CoreOpId`.
- `visibility` — `public` / `internal`.
- `classification` — `layer` is one of `primitive`, `runtime`, `semantic_stdlib`,
  `normal_stdlib`, `target_raw`.
- `binding` — `policy` is `required` / `optional` / `forbidden`. `optional`
  must include a `reason`.
- `signature` — `inputs` (list of `TypeExpr`), `receiver_index` (optional,
  must be a valid index into `inputs`), `outputs` (list of `TypeExpr`),
  `generic_params`, `constraints`.
- `semantics` — `const_evaluable`, overflow/NaN/trap rules, `equivalence`
  strategy for differential tests.
- `effect` — orthogonal attribute set (`memory`, `allocates`, `may_trap`,
  `noreturn`, `external_io`, `nondeterminism`, `atomic`, `volatile`).
- `inline` — `policy` (`never`, `hint`, `always`). `always` is a strong hint,
  not a semantic guarantee.
- `lowering` — `kind` (`normal_call`, `mir_op`, `runtime_call`, `target_intrinsic`)
  plus variant-specific payload.
  - `normal_call` uses `[fallback]` with `implementation_symbol`.
  - `mir_op` uses `[lowering.mir]` with `opcode` / `operation`.
  - `runtime_call` uses `[lowering.runtime]` with `function`, `abi`,
    optional `interface` / `version`.
  - `target_intrinsic` uses `[lowering.target]` with `target_family`,
    `target_id`, `required_capabilities`, `required_target_features`.
- `specializations` — optional `[[operations.specializations]]` array. Each
  specialization has `priority`, `when` (condition table), and a full `lowering`
  variant. Selection is highest unique priority; ambiguity is a schema error.
  If no specialization matches, the default `lowering` is used.
- `fallback` — `implementation_symbol` (stable internal path, not a public path)
  and `required`.

Compile-local `FunctionId` must not be persisted in the registry.

### Type expressions

`inputs`/`outputs` use `TypeExpr` tables. The `kind` discriminator determines
the shape:

| `kind` | Fields | Example |
|--------|--------|---------|
| `ref` | `name` (type id in `[[types]]`), optional `args` | `{ kind = "ref", name = "vec", args = [{ kind = "var", name = "T" }] }` |
| `primitive` | `name` (`i32`, `i64`, `f32`, `f64`, `bool`, `unit`, ...) | `{ kind = "primitive", name = "i32" }` |
| `var` | `name` (generic parameter) | `{ kind = "var", name = "T" }` |
| `tuple` | `elements` (list of `TypeExpr`) | `{ kind = "tuple", elements = [...] }` |
| `function` | `params` (list of `TypeExpr`), `result` (TypeExpr) | `{ kind = "function", params = [...], result = {...} }` |

`()` in `std/manifest.toml` is normalized to `unit` (`{ kind = "primitive", name = "unit" }`).
`String` in `std/manifest.toml` maps to `type_id = "string"` (`{ kind = "ref", name = "string" }`).
Raw `v128` in `std::wasm` maps to `type_id = "wasm.v128"`.

### SignatureEntry vs CoreOpRegistry

`SignatureEntry` is the compile-time record attached to a `FunctionId`. It
contains at minimum:

- the function signature (after typechecking / monomorphization)
- an optional `core_op_id` link

`CoreOpId` metadata lives in `data/core-ops.toml` (the `CoreOpRegistry`).
Compiler components look up:

```text
FunctionId
→ SignatureEntry
→ CoreOpId
→ CoreOpRegistry entry
```

This avoids duplicating effect, lowering, fallback, and inline policy into every
`SignatureEntry`. A compiler may cache derived metadata, but the cache is not
authoritative.

### Validation

`[validation]` lists the checks the generator/checker runs. Each checker is
conditional on `visibility` and `binding`:

- `check_unreferenced_required_bindings` — `visibility = "public"` and
  `binding.policy = "required"` operations must be referenced by
  `std/manifest.toml` via `core_op_id`.
- `check_public_binding_collisions` — the same public symbol must not map to
  more than one `CoreOpId`.
- `check_signature_compat` — manifest `params` / `returns` must be positionally
  compatible with `core-ops` `signature`.
- `check_effect_lowering_consistency` — `lowering.kind` and `effect` must agree.
- `check_fallback_resolvable` — operations with `fallback.required = true` must
  have `fallback.implementation_symbol`.
- `check_specialization_ambiguity` — no two specializations of the same
  operation may match the same conditions with the same priority.

## Status

Current `schema_version = 4` entries are **scaffold** examples covering
`string.starts_with`, `string.ends_with`, `panic`, portable SIMD, and a raw
`std::wasm` load. Resolver / typechecker / MIR / docs generators do not consume
this file today. The schema and operation set are non-normative while ADR-042
remains PROPOSED.
