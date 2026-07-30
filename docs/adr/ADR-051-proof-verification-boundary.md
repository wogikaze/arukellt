# ADR-051: Proof verification boundary

- Status: ACCEPTED
- Date: 2026-07-31

## Context

Arukellt needs formal verification in two different places: contracts for user programs and correctness checks for compiler transformations. Embedding a prover in the self-hosted compiler would enlarge the trusted computing base and couple solver availability to every compile target. The current compiler also has no general host process-spawn API, so the self-hosted CLI cannot directly supervise Why3 or SMT solvers.

The existing `where` surface already owns scoped helper value bindings for function clause groups. Reusing it for proof contracts would mix execution scope with logical obligations and make parsing ambiguous.

## Decision

Arukellt adopts a hybrid architecture.

1. Proof annotations are part of the language surface and are retained through the typed frontend.
2. Function contracts use a dedicated `proof { ... }` block after the return type and before the body. The initial clauses are `requires:` and `ensures:`; `ensures` binds `result` to the declared return type.
3. Contract expressions must type-check as `bool`. They do not become runtime statements or enter backend lowering.
4. The compiler emits a versioned, target-independent Proof IR artifact.
5. Host-side tooling validates that artifact and translates it to an external verifier backend.
6. Why3 plus an SMT solver is the initial backend direction. The backend is not part of the language semantics and may be replaced.
7. The compiler, Proof IR producer, backend translator, solver, and any explicitly axiomatized declarations are recorded as trusted components. Successful proof does not by itself prove CoreHIR-to-MIR or MIR-to-Wasm correctness.
8. Compiler correctness remains a separate track: typed MIR validation, translation validation, and proof of selected lowering or optimization passes.

## Initial contract surface

The first reserved contract kinds are `requires`, `ensures`, `invariant`, `assert`, and `decreases`. The first parser slice accepts function-level `requires` and `ensures`. Loop invariants, assertions, decreases clauses, purity rules, and old-value syntax require follow-up design. Proof IR keeps an explicit result name rather than relying on backend-specific syntax.

## Proof IR contract

`schemas/proof-ir-v1.schema.json` is the machine-readable interchange schema. `scripts/proof/ir.py` is the dependency-free executable boundary validator used before any backend is invoked. Schema changes require a version increment; consumers must reject unknown versions and unknown fields.

The initial `body` field is intentionally opaque. It carries a stable reference to a typed CoreHIR body until the expression subset is specified. A later version will replace or refine it with a normalized verification expression language.

## Consequences

The compiler can add proof support without linking a solver into bootstrap artifacts. CI can validate artifact structure on all lanes and run solver-backed checks only where dependencies are installed. Proof success remains honest about the trusted frontend and backend boundary.

The current implementation parses and type-checks function contracts but does not yet prove them, generate verification conditions, emit contracts from typed CoreHIR, check purity, or claim end-to-end compiler correctness.
