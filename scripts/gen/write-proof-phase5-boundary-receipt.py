#!/usr/bin/env python3
"""Write the final hash-bound proof Phase 5 boundary receipt."""
from __future__ import annotations
import argparse, hashlib, json, sys
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]
FILES = (
    "docs/data/proof-capabilities-v2.json",
    "scripts/proof/capabilities_v2.py",
    "scripts/proof/verified_core_interface.py",
    "scripts/proof/loop_semantics.py",
    "scripts/proof/aggregate_types.py",
    "scripts/proof/aggregate_semantics.py",
    "scripts/proof/aggregate_instructions.py",
    "scripts/proof/aggregate_compat_v5.py",
    "scripts/proof/typed_admission_v5_final.py",
    "scripts/proof/typed_corehir_v2.py",
    "scripts/proof/typed_corehir_v2_convert.py",
    "scripts/proof/typed_corehir_v2_convert_final.py",
    "scripts/proof/proof_phase4_vc.py",
    "scripts/proof/proof_phase5_vc.py",
    "scripts/proof/smtlib_typed_v5_final.py",
    "scripts/proof/typed_verified_core_receipt_v5.py",
    "scripts/check/check-proof-phase3-modular.py",
    "scripts/check/check-proof-phase5-boundary.py",
    "scripts/tests/test_loop_proof_v4.py",
    "scripts/tests/test_proof_phase5_final.py",
    "scripts/tests/test_typed_corehir_v2_aggregates.py",
)
CHECKS = [
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
    "typed-corehir-v2-aggregate-lowering",
    "semantic-admission-before-SMT",
]

def sha(path: Path) -> str:
    d = hashlib.sha256(); d.update(path.read_bytes()); return d.hexdigest()

def main() -> int:
    parser = argparse.ArgumentParser(); parser.add_argument("--output", type=Path, default=ROOT/".build/proof/proof-phase5-boundary.json"); args = parser.parse_args()
    files = []
    for relative in FILES:
        path = ROOT/relative
        if not path.is_file(): raise ValueError(f"Phase 5 boundary file missing: {relative}")
        files.append({"path": relative, "sha256": sha(path)})
    value = {
        "schema": "arukellt-proof-phase-boundary", "schema_version": 1, "phase": 5, "status": "enforced",
        "capability_profile": "proof-phases-0-5@2", "source_schema": "arukellt-typed-corehir@2",
        "target_schema": "arukellt-verified-core@1", "converter": "arukellt-typed-corehir-converter-v5",
        "semantic_validator": "typed_admission_v5_final.py@5", "solver_adapter": "smtlib_typed_v5_final.py@5",
        "aggregate_encoding": "arukellt-smt-datatype-v1", "semantic_checks": CHECKS,
        "failure_action": "reject-before-SMT-generation", "files": files,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True); args.output.write_text(json.dumps(value, indent=2, sort_keys=True)+"\n", encoding="utf-8")
    print(f"proof-phase5-boundary: PASS: files={len(files)} output={args.output}"); return 0

if __name__ == "__main__":
    try: raise SystemExit(main())
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        print(f"proof-phase5-boundary: FAIL: {exc}", file=sys.stderr); raise SystemExit(1)
