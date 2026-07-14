# Core Ops Registry

> Registry path: [`data/core-ops.toml`](../../data/core-ops.toml)
> Status: designated future SSOT; currently `status = "scaffold"` and not consumed by the compiler
> Proposed design: [ADR-042](../adr/ADR-042-intrinsic-layer-separation.md) D5
> Migration owner: [issue #798](../../issues/open/798-adr-042-semantic-operation-registry-migration.md)

## Role

`data/core-ops.toml` is the **designated future single source of truth** for
semantic types, `CoreOpId`, effect, inline policy, lowering, fallback, signature,
and exposure.

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
```

The `core_op_id` / `type_id` references shown above are syntactically accepted
and used by the example entries, but `core-ops.toml` is not yet consumed by the
compiler. Issue #798 owns adoption after ADR-042 acceptance.

## Registry schema

The current schema is `schema_version = 3`. See `data/core-ops.toml` for the
reference entries. Each operation has:

- `id` — canonical `CoreOpId`.
- `exposure` — `public` / `internal` / `runtime` / `target_raw`.
- `binding_required` — whether `std/manifest.toml` must hold a public binding
  for this operation (only meaningful for `public`).
- `signature` — `inputs` (receiver-neutral input list), `receiver_index`,
  `outputs`, `generic_params`, `constraints`.
- `semantics` — `const_evaluable`, overflow/NaN/trap rules.
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
    `target_id`, `required_features`.
- `specializations` — optional `[[operations.specializations]]` array for
  portable operations that can lower to a target instruction under conditions
  such as `portable_simd_lowering = "NativeSimd"`.
- `fallback` — `implementation_symbol` (stable internal path, not a public path)
  and `required`.

Compile-local `FunctionId` must not be persisted in the registry.

### Validation

`[validation]` lists the checks the generator/checker runs. Each checker is
conditional on `operations.exposure` and `operations.binding_required`:

- `check_unreferenced_public_bindings` — `public` and `binding_required = true`
  operations must be referenced by `std/manifest.toml` via `core_op_id`.
- `check_public_binding_collisions` — the same public symbol must not map to
  more than one `CoreOpId`.
- `check_signature_compat` — manifest `params` / `returns` must be positionally
  compatible with `core-ops` `signature`.
- `check_effect_lowering_consistency` — `lowering.kind` and `effect` must agree.
- `check_fallback_resolvable` — operations with `fallback.required = true` must
  have `fallback.implementation_symbol`.

## Status

Current `schema_version = 3` entries are **scaffold** examples covering
`string.starts_with`, `panic`, portable SIMD, and a raw `std::wasm` load.
Resolver / typechecker / MIR / docs generators do not consume this file today.
The schema and operation set are non-normative while ADR-042 remains PROPOSED.
