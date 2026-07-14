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
  `target_raw`. `normal_stdlib` is **not** a CoreOpRegistry layer; normal stdlib
  functions have no `core_op_id` once migrated.
- `binding` — `policy` is `required` / `optional` / `forbidden`. `optional`
  must include `reason` and a `tracking_issue` / RFC reference.
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
  - `runtime_call` uses `[lowering.runtime]` which is a discriminated union:
    - `kind = "internal"` — `symbol` + `abi_version`
    - `kind = "wit"` — `package` + `interface` + `function` + `version`
    - `kind = "native"` — `backend` + `symbol` + `abi_version`
  - `target_intrinsic` uses `[lowering.target]` with `target_family`,
    `target_id`, `required_capabilities`, `required_target_features`.
    `target_id` is a **backend-owned handler key**, not a literal opcode.
    The handler owns the argument interpretation (e.g. `wasm.v128.load` adds
    `address` and `offset` to form an effective linear-memory address).
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
| `primitive` | `name` (`i32`, `i64`, `f32`, `f64`, `bool`, `unit`, `char`, `u8`, `i8`, `u16`, `i16`, `u32`, `u64`, `never`, ...) | `{ kind = "primitive", name = "i32" }` |
| `var` | `name` (generic parameter) | `{ kind = "var", name = "T" }` |
| `tuple` | `elements` (list of `TypeExpr`) | `{ kind = "tuple", elements = [...] }` |
| `function` | `params` (list of `TypeExpr`), `result` (TypeExpr) | `{ kind = "function", params = [...], result = {...} }` |

`()` in `std/manifest.toml` is normalized to `unit` (`{ kind = "primitive", name = "unit" }`).
`String` in `std/manifest.toml` maps to `type_id = "string"` (`{ kind = "ref", name = "string" }`).
Raw `v128` in `std::wasm` maps to `type_id = "wasm.v128"` (`{ kind = "ref", name = "wasm.v128" }`).

`receiver_index` is optional and, if present, must satisfy
`0 <= receiver_index < len(inputs)`. It identifies the receiver argument for
method-style dispatch; the `inputs` array itself remains receiver-independent.

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

### Manifest type syntax → TypeExpr

`std/manifest.toml` `params` / `returns` are still strings. The checker parses
and normalizes them to `TypeExpr` using one shared grammar. The grammar is
Arukellt type syntax restricted to the `TypeExpr` shapes.

```text
Type       ::= Ref | Tuple | Function | Primitive
Ref        ::= IDENT [ '<' Type (',' Type)* '>' ]
Tuple      ::= '(' [ Type (',' Type)* ] ')'
Function   ::= 'fn' '(' [ Type (',' Type)* ] ')' '->' Type
Primitive  ::= 'i32' | 'i64' | 'f32' | 'f64' | 'bool' | 'char'
             | 'u8' | 'i8' | 'u16' | 'i16' | 'u32' | 'u64'
             | 'unit' | 'never'
IDENT      ::= [A-Za-z_][A-Za-z0-9_]*
```

The empty tuple `()` is normalized to `unit`, not a zero-element `tuple`.
`String` in `std/manifest.toml` maps to `type_id = "string"` (`{ kind = "ref", name = "string" }`).
`v128` in `std::wasm` maps to `type_id = "wasm.v128"`.
`v128` in `std::simd` maps to the appropriate portable SIMD type (e.g. `simd.i32x4`).

`ref` generic arity is validated against the `[[types]]` entry's `generic_params`.
`var` may only appear in `generic_params` scope or in the `args` of a `ref` that
belongs to a generic `type_id`.

### Constraints

`signature.constraints` is a list of `Constraint` objects. The exact schema is
scaffold; the `kind` discriminator determines the shape:

| `kind` | Fields | Example |
|--------|--------|---------|
| `trait` | `trait` (name), `params` (list of `TypeExpr`) | `{ kind = "trait", trait = "Stringable", params = [{ kind = "var", name = "T" }] }` |
| `type_eq` | `lhs` / `rhs` (`TypeExpr`) | `{ kind = "type_eq", lhs = {...}, rhs = {...} }` |
| `capability` | `capability` (name) | `{ kind = "capability", capability = "wasm_raw_v128" }` |

Empty `constraints = []` is allowed for non-generic operations.

### Binding policy combinations

| `visibility` | `binding.policy` | Meaning |
|--------------|------------------|---------|
| `public` | `required` | At least one `std/manifest.toml` binding with this `core_op_id` must exist. |
| `public` | `optional` | Binding is optional; `reason` and `tracking_issue` are required. |
| `public` | `forbidden` | Invalid combination (public operations must be bindable). |
| `internal` | `forbidden` | Normal form for internal operations. |
| `internal` | `required` | Invalid (internal operations have no manifest binding). |
| `internal` | `optional` | Invalid unless a clear use case is documented. |

### Validation layers

Validation is split into three layers. `data/core-ops.toml` and
`scripts/check/check-core-ops.py` cover the first layer. The second and third
layers require compiler or runtime support and are documented as requirements
for `check-core-ops.py --production-readiness`.

| Layer | Owner | Scope |
|-------|-------|-------|
| Python schema checker | `scripts/check/check-core-ops.py` | TOML structure, ID uniqueness, `TypeExpr` arity and manifest parse, binding policy, public reference, lowering payload closure, specialization static overlap |
| Compiler-aware validator | compiler (future) | fallback symbol resolution, call graph and cycles, Ark signature compatibility, effect/lowering consistency, target handler registry lookup |
| Differential test | runtime / test harness (future) | fallback vs specialized lowering produce equivalent observable results and side effects |

### Validation

`[validation]` lists the invariant checks the Python schema checker runs. These
are not user-configurable toggles; the checker always runs them and the file
must contain all required keys set to `true`.

- `check_unreferenced_required_bindings` — `visibility = "public"` かつ
  `binding.policy = "required"` の operation が `std/manifest.toml` から `core_op_id` で参照される。
- `check_public_binding_collisions` — 同じ public symbol が複数の `CoreOpId` に対応しない。
- `check_signature_compat` — manifest `params` / `returns` を `TypeExpr` に parse し、
  `data/core-ops.toml` の `signature` と位置・型で対応する。
- `check_effect_lowering_consistency` — `lowering.kind` と `effect` が矛盾しない。
- `check_binding_field_consistency` — `visibility` / `binding.policy` / `classification` の
  組合せが上記の表に合う。
- `check_forbidden_bindings` — `public` + `forbidden` や `internal` + `required` など
  無効な組合せを検出する。
- `check_fallback_resolvable` — `fallback.required = true` な operation に
  `fallback.implementation_symbol` が設定されている。`example.invalid.*` 接頭辞は解決できない。
- `check_fallback_no_cycle` — fallback 呼び出しグラフに閉路がない（compiler-aware validator）。
- `check_fallback_signature_compat` — fallback のシグネチャが CoreOp の signature と
  互換である（compiler-aware validator）。
- `check_specialization_ambiguity` — 同じ target configuration に対して最も高い priority の
  specialization が一意である。`when` 内の key は閉じた集合（`backend`, `target_family`,
  `portable_simd_lowering`, `wasm_raw_v128`, `wasm_relaxed_simd`）に限定する。

### Scaffold-exit criteria

`data/core-ops.toml` が `status = "scaffold"` から production 状態へ移行する前に
次を満たす:

- `example.invalid.*` fallback シンボルが 0 件である
- `python3 scripts/check/check-core-ops.py --strict` が PASS
- compiler-aware validator により全 `required` fallback が解決可能
- `std/manifest.toml` の全 `core_op_id` / `type_id` 参照が有効
- compiler が `CoreOpRegistry` を消費する
- `python3 scripts/check/check-core-ops.py --production-readiness` が PASS

### Checker commands

- `python3 scripts/check/check-core-ops.py` — structural + manifest-semantic check
  (allows `example.invalid.*` placeholders when `status = "scaffold"`).
- `python3 scripts/check/check-core-ops.py --strict` — structural + manifest-semantic check
  that rejects `example.invalid.*` placeholders.
- `python3 scripts/check/check-core-ops.py --production-readiness` — strict check plus
  `status = "production"` gate and explicit acknowledgement that compiler-aware
  validator receipts are still required.

## Status

Current `schema_version = 4` entries are **scaffold** examples covering
`string.starts_with`, `string.ends_with`, `panic`, portable SIMD, and a raw
`std::wasm` load. Resolver / typechecker / MIR / docs generators do not consume
this file today. The schema and operation set are non-normative while ADR-042
remains PROPOSED.
