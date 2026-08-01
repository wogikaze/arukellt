# ADR-051: Proof verification boundary

ステータス: **ACCEPTED** — 言語組み込み契約を型付き CoreHIR に保持し、Proof IR 経由で外部 verifier へ渡す

決定日: 2026-07-30

---

## Context

Arukellt needs formal verification in two different places: contracts for user programs and correctness checks for compiler transformations. Embedding a prover in the self-hosted compiler would enlarge the trusted computing base and couple solver availability to every compile target. The current compiler also has no general host process-spawn API, so the self-hosted CLI cannot directly supervise Why3 or SMT solvers.

The existing `where` surface already owns scoped helper value bindings for function clause groups. Reusing it for proof contracts would mix execution scope with logical obligations and make parsing ambiguous.

## Decision

Arukellt adopts a hybrid architecture.

1. Proof annotations are part of the language surface and are retained through the typed frontend.
2. Function contracts use a dedicated `proof { ... }` block after the return type and before the body. The initial clauses are `requires:` and `ensures:`; `ensures` binds `result` to the declared return type.
3. Contract expressions must type-check as `bool`. They are retained in a proof-only CoreHIR table and do not become runtime statements or enter MIR/backend lowering.
4. The compiler emits a versioned, target-independent Proof IR artifact. The initial diagnostic surface is `--dump-phases proof-ir`.
5. Host-side tooling validates that artifact and translates it to an external verifier backend.
6. Why3 plus an SMT solver is the initial backend direction. The backend is not part of the language semantics and may be replaced.
7. The compiler, Proof IR producer, backend translator, solver, and any explicitly axiomatized declarations are recorded as trusted components. Successful proof does not by itself prove CoreHIR-to-MIR or MIR-to-Wasm correctness.
8. Compiler correctness remains a separate track: typed MIR validation, translation validation, and proof of selected lowering or optimization passes.

## Promotion hard gates

Proof support remains experimental, and a release may not claim `proof-required`, until all of the following are true:

- every major compiler boundary emits a versioned artifact with an independent fail-closed validator;
- backends consume explicit type, ABI, nullability, representation, and layout data and do not reconstruct them from names or stack history;
- VerifiedCore is a typed representation, not an opaque body index or mutable side table;
- optimizer passes covered by the verified profile have translation validation, with source and target artifacts bound into a receipt;
- every solver result carries a versioned TrustManifest identifying the producer, translator, solver binary, semantic profile, limits, assumptions, and trusted components;
- the legacy large mutable table API is removed from the verified path rather than wrapped or mirrored indefinitely;
- a `proof-required` release cannot pass without a valid `status=proved` ProofReceipt bound by SHA-256 to both Proof IR and its TrustManifest.

These are hard gates, not documentation goals. A partially implemented item must remain visibly unfulfilled in release policy and maturity metadata.

## Initial contract surface

The first reserved contract kinds are `requires`, `ensures`, `invariant`, `assert`, and `decreases`. The first parser slice accepts function-level `requires` and `ensures`. Loop invariants, assertions, decreases clauses, purity rules, and old-value syntax require follow-up design. Proof IR keeps an explicit result name rather than relying on backend-specific syntax.

## Versioned proof artifacts

- `schemas/proof-ir-v1.schema.json` defines the compiler-to-verifier interchange artifact.
- `schemas/trust-manifest-v1.schema.json` defines the complete trust boundary for one solver invocation.
- `schemas/proof-receipt-v1.schema.json` binds the solver outcome to Proof IR and TrustManifest digests.
- `schemas/proof-release-policy-v1.schema.json` declares whether a release is `proof-optional` or `proof-required`.

`scripts/proof/ir.py` and `scripts/proof/trust.py` are dependency-free executable validators. Consumers reject unknown versions and unknown fields. `scripts/check/check-proof-release.py` is fail-closed: a proof-required policy must list at least one receipt, every file must exist and validate, digest bindings must match, and every receipt must have `status=proved`.

Contract expressions are rendered from CoreHIR as deterministic S-expressions. The initial `body` field is intentionally opaque and carries a CoreHIR body root. This is a transitional v1 representation and does not satisfy the typed VerifiedCore promotion gate.

## Consequences

The compiler can add proof support without linking a solver into bootstrap artifacts. CI can validate artifact structure on all lanes and run solver-backed checks only where dependencies are installed. Proof success remains honest about the trusted frontend and backend boundary.

The current implementation parses, type-checks, retains, and serializes function contracts and defines TrustManifest/ProofReceipt/release-policy boundaries. It does not yet prove contracts, generate verification conditions, invoke Why3, check proof-expression purity, model machine-integer overflow, provide typed VerifiedCore bodies, translation-validate optimizer passes, remove the legacy mutable CoreHIR table path, or claim end-to-end compiler correctness.
