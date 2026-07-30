# Proof contracts

Status: experimental frontend surface.

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

The `proof` block is separate from the existing `where` value-binding surface. Contracts are stored as dedicated AST annotations and do not become runtime statements or affect Wasm emission.

This first frontend slice performs parsing and type checking only. It does not yet generate verification conditions, emit Proof IR from CoreHIR, invoke Why3, check purity, or prove that the function body satisfies its contracts. Those capabilities remain external-tooling work under ADR-051.
