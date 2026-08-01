# ADR-051: Formal verification hard gates

Status: **ACCEPTED**

Date: 2026-08-01

## Decision

Arukellt uses a hybrid verification architecture. Proof-facing syntax may live in the language, but semantics, verification-condition generation, solver execution, translation validation, and receipt checking remain independently executable boundaries.

A release may claim `proof-required` only after all seven gates below are satisfied:

1. Major compiler boundaries emit versioned artifacts with independent fail-closed validators.
2. Backends consume explicit type, ABI, nullability, representation, and layout data; they do not reconstruct these facts from names, stack history, or fixed offsets.
3. VerifiedCore is a typed representation with typed bodies, signatures, locals, contracts, and control flow. An opaque body index is insufficient.
4. Optimizer passes in the verified profile emit source/target-bound translation-validation receipts.
5. Every solver result carries a TrustManifest identifying the producer, translator, solver executable, semantic profile, limits, assumptions, and trusted components.
6. The legacy large mutable table API is absent from the verified path rather than wrapped or mirrored indefinitely.
7. A `proof-required` release cannot pass without a valid `status=proved` ProofReceipt bound to its subject, TrustManifest, and solver output.

## Artifact set

- `arukellt-verified-core` v1: typed proof subject with explicit representation and ABI data.
- `arukellt-trust-manifest` v1: exact trust boundary for one verification run.
- `arukellt-proof-receipt` v1: solver result bound by SHA-256.
- `arukellt-translation-validation` v1: optimizer-pass result bound to source and target artifacts.
- `arukellt-proof-release-policy` v1: release mode, hard-gate state, and required receipts.

All validators reject unknown fields and unknown schema versions. JSON artifact digests are over the exact artifact bytes; solver output digests are over exact output bytes.

## Current state

The artifact schemas, independent validators, TrustManifest, ProofReceipt, translation receipt, and fail-closed release policy are implemented as a host-side foundation.

The repository policy remains `proof-optional`. The following gates remain false until compiler integration is complete:

- versioned artifacts at every major boundary;
- explicit backend type/ABI/layout consumption across all backends;
- compiler emission of typed VerifiedCore;
- optimizer-pass integration with translation validation;
- removal of legacy mutable tables from the verified path.

The presence of a schema or fixture is not evidence that the corresponding compiler producer is complete.
