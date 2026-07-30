# Proof contracts

Status: experimental frontend and artifact surface.

Arukellt function contracts use a dedicated `proof` block after the return type and before the function body:

```ark
fn nonnegative_identity(x: i32) -> i32 proof {
    requires: x >= 0
    ensures: result >= x
} {
    x
}
```

`requires` describes a precondition and is type-checked in the function parameter scope. `ensures` describes a postcondition and additionally binds `result` to the declared return type. Every contract expression must have type `bool`; a non-boolean expression is a compile error.

The initial surface supports top-level functions only. A `proof` block on an impl method is rejected until Proof IR has a stable method identity, receiver model, and method-body mapping.

The `proof` block is separate from the existing `where` value-binding surface. Contracts are stored as dedicated AST annotations, lowered into a proof-only CoreHIR table, and excluded from function body roots and MIR lowering. They therefore do not become runtime statements or affect Wasm emission.

A versioned debug artifact can be inspected with:

```sh
scripts/run/arukellt-selfhost.sh compile example.ark --dump-phases proof-ir
```

The compiler prints an `arukellt-proof-ir` schema-v1 JSON object after the `=== PROOF IR ===` marker. Contract expressions are normalized to deterministic S-expressions such as `(ge (ident "x") (int 0))`. The host-side structural validator is:

```sh
python3 scripts/check/check-proof-ir.py artifact.json
```

The current slice does not yet generate verification conditions, invoke Why3, enforce proof-expression purity, model machine-integer overflow, or prove that the function body satisfies its contracts. The Proof IR body remains an opaque CoreHIR body reference until the verification expression and statement semantics are fixed.
