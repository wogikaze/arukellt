# Proof phases 3–5

This document fixes the proof semantics implemented by the Phase 3–5 boundary. The proof pipeline is fail-closed: unsupported composition is rejected before SMT generation.

## Phase 3 — modular direct calls

A direct proof call consumes only the callee interface: signature, ABI, `requires`, and `ensures`. The caller must prove each callee `requires`; the callee `ensures` become post-call facts. The exact interface is canonical-JSON encoded and SHA-256 bound in the call instruction as `callee_interface_sha256`. Changing the callee contract without rebinding the caller therefore fails admission.

Indirect calls and recursive proof-call cycles are outside Phase 3 and are rejected.

## Phase 4 — annotated loops

A proof loop has one or more boolean invariants and exactly one integer `decreases` expression. Verification is rule-based, not bounded unrolling. The generated obligations are:

- invariant initiation at loop entry;
- invariant preservation across the explicit backedge;
- exit reasoning from invariant plus negated condition;
- non-negative decreases value;
- strict decrease across the backedge.

Unannotated cycles are rejected. Nested annotated proof loops, proof `break` exits, and proof `continue` branch edges are outside the initial Phase 4 subset and are rejected rather than approximated.

## Phase 5 — pure aggregates

Phase 5 adds pure algebraic values. The proof-facing source artifact is TypedCoreHIR v2. Aggregate TypeIds are explicit metadata; human-readable type names are identity/display information only.

Supported aggregate types are:

- tuple: ordered `elements` TypeIds;
- struct/record: ordered `fields` with explicit TypeIds;
- enum: ordered variants with explicit non-negative discriminants and payload TypeIds;
- `Option` and `Result`: ordinary enum instances, with no name-based semantic special case.

Supported proof operations are `construct`, `project`, `is_variant`, `variant_payload`, and equality/inequality on values with equal aggregate TypeId.

Recursive aggregate type graphs are outside Phase 5. Aggregate-bearing functions are initially straight-line; aggregate calls and aggregate loops are rejected. Scalar Phase 3 calls and scalar Phase 4 loops remain supported in the same module when they do not cross the aggregate boundary.

### SMT encoding

Aggregate SMT encoding is versioned as `arukellt-smt-datatype-v1`. Sort, constructor, selector, tester, and payload-selector identities are derived only from TypeId and numeric member/variant indices. Renaming a source type, field, or variant without changing its explicit typed structure does not change the solver input.

The boundary receipt binds the capability profile, TypedCoreHIR v2 schema/converter, semantic validators, loop and call validators, datatype renderer, CLI entrypoints, tests, and release toolchain components by SHA-256.
