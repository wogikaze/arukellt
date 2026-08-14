#!/usr/bin/env python3
"""Write hash-bound typed VerifiedCore boundary evidence through proof Phase 5."""
from __future__ import annotations
import argparse, hashlib, json, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BOUNDARY_FILES = (
    "docs/data/proof-capabilities-v2.json",
    "scripts/proof/capabilities_v2.py",
    "scripts/proof/verified_core.py",
    "scripts/proof/verified_core_ext.py",
    "scripts/proof/verified_core_interface.py",
    "scripts/proof/verified_core_typed.py",
    "scripts/proof/verified_core_typed_impl.py",
    "scripts/proof/verified_core_typed_phase4.py",
    "scripts/proof/typed_admission_v4.py",
    "scripts/proof/typed_admission_v5.py",
    "scripts/proof/loop_semantics.py",
    "scripts/proof/aggregate_types.py",
    "scripts/proof/aggregate_semantics.py",
    "scripts/proof/aggregate_instructions.py",
    "scripts/proof/typed_corehir_typed_convert.py",
    "scripts/proof/typed_corehir_phase4_convert.py",
    "scripts/proof/proof_phase4_vc.py",
    "scripts/proof/proof_phase5_vc.py",
    "scripts/proof/smtlib_typed_v1.py",
    "scripts/proof/typed_verified_core_receipt_v5.py",
    "scripts/check/check-proof-phase3-modular.py",
    "scripts/check/check-proof-phase5-boundary.py",
    "scripts/tests/test_loop_proof_v4.py",
    "scripts/tests/test_proof_phase5_aggregates.py",
)
SEMANTIC_CHECKS = [
    "operator-arity-and-TypeId-preservation",
    "contract-root-typing",
    "result-return-TypeId-equality",
    "parameter-signature-local-bijection",
    "constant-payload-typing",
    "contract-kind-expression-id-uniqueness",
    "straight-line-instruction-typing",
    "acyclic-cfg-edge-typing",
    "direct-call-contract-typing",
    "exact-callee-interface-sha256-binding",
    "recursive-call-rejection",
    "annotated-loop-invariant-typing",
    "loop-initiation-preservation-exit-vc",
    "strict-decreases-termination-vc",
    "unannotated-cycle-rejection",
    "pure-aggregate-type-metadata",
    "aggregate-constructor-projection-typing",
    "enum-variant-payload-typing",
    "deterministic-smt-datatype-v1",
    "recursive-aggregate-rejection",
    "semantic-admission-before-SMT",
]

def sha256(path: Path) -> str:
    digest = hashlib.sha256(); digest.update(path.read_bytes()); return digest.hexdigest()

def main() -> int:
    parser = argparse.ArgumentParser(); parser.add_argument("--output", type=Path, default=ROOT / ".build" / "proof" / "typed-verified-core-boundary-v5.json"); args = parser.parse_args()
    files = []
    for relative in BOUNDARY_FILES:
        path = ROOT / relative
        if not path.is_file(): raise ValueError(f"typed VerifiedCore v5 boundary file missing: {relative}")
        files.append({"path": relative, "sha256": sha256(path)})
    document = {
        "schema": "arukellt-typed-verified-core-boundary", "schema_version": 1, "status": "enforced",
        "proof_phase": 5, "capability_profile": "proof-phases-0-5@2",
        "source_schema": "arukellt-typed-corehir@1", "target_schema": "arukellt-verified-core@1",
        "converter": "arukellt-typed-corehir-converter-v4", "semantic_validator": "typed_admission_v5.py@5",
        "solver_adapter": "smtlib_typed_v1.py@1", "aggregate_encoding": "arukellt-smt-datatype-v1",
        "semantic_checks": SEMANTIC_CHECKS, "failure_action": "reject-before-SMT-generation", "files": files,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True); args.output.write_text(json.dumps(document, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"typed-verified-core-boundary-v5: PASS: files={len(files)} output={args.output}")
    return 0

if __name__ == "__main__":
    try: raise SystemExit(main())
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        print(f"typed-verified-core-boundary-v5: FAIL: {exc}", file=sys.stderr); raise SystemExit(1)
