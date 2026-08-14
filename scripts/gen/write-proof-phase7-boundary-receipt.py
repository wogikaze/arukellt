#!/usr/bin/env python3
"""Write the hash-bound proof Phase 7 read-only-memory boundary receipt."""
from __future__ import annotations
import argparse
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
FILES = (
    "docs/data/proof-capabilities-v3.json",
    "scripts/proof/capabilities_v3.py",
    "scripts/gen/write-proof-phase6-boundary-receipt.py",
    "scripts/check/check-proof-phase6-boundary.py",
    "schemas/typed-corehir-v3.schema.json",
    "scripts/proof/typed_corehir_v3.py",
    "scripts/proof/typed_corehir_v3_convert.py",
    "scripts/proof/readonly_memory_semantics.py",
    "scripts/proof/readonly_memory_compat_v7.py",
    "scripts/proof/readonly_memory_phase7_limits.py",
    "scripts/proof/typed_admission_v7.py",
    "scripts/proof/proof_phase7_vc.py",
    "scripts/proof/smtlib_typed_v7.py",
    "scripts/tests/test_proof_phase7_readonly_memory.py",
    "scripts/tests/test_typed_corehir_v3_machine_memory.py",
    "scripts/gen/write-proof-phase7-boundary-receipt.py",
    "scripts/check/check-proof-phase7-boundary.py",
)
CHECKS = [
    "phase6-boundary-hash-chain",
    "typed-corehir-v3-source-profile",
    "typed-corehir-v3-machine-memory-lowering",
    "TypeId-bound-reference-descriptors",
    "explicit-reference-nullability",
    "display-name-independent-reference-semantics",
    "reference-identity-without-distinctness-assumption",
    "read-only-object-field-load",
    "array-slice-length-read",
    "array-slice-index-read",
    "nonnull-dereference-vc",
    "array-index-lower-bound-vc",
    "array-index-upper-bound-vc",
    "memory-mutation-allocation-rejection",
    "aggregate-contained-reference-rejection",
    "memory-loop-call-composition-rejection",
    "memory-dependent-decreases-rejection",
    "semantic-admission-before-SMT",
]


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    digest.update(path.read_bytes())
    return digest.hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, default=ROOT / ".build/proof/proof-phase7-boundary.json")
    args = parser.parse_args()
    files = []
    for relative in FILES:
        path = ROOT / relative
        if not path.is_file():
            raise ValueError(f"Phase 7 boundary file missing: {relative}")
        files.append({"path": relative, "sha256": sha256(path)})
    value = {
        "schema": "arukellt-proof-phase-boundary",
        "schema_version": 1,
        "phase": 7,
        "status": "enforced",
        "capability_profile": "proof-phases-0-7@3",
        "source_schema": "arukellt-typed-corehir@3",
        "target_schema": "arukellt-verified-core@1",
        "converter": "arukellt-typed-corehir-converter-v7",
        "semantic_validator": "typed_admission_v7.py@7",
        "solver_adapter": "smtlib_typed_v7.py@7",
        "memory_model": "arukellt-readonly-heap-v1",
        "memory_encoding": "arukellt-readonly-heap-smt-v1",
        "semantic_checks": CHECKS,
        "failure_action": "reject-before-SMT-generation",
        "files": files,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"proof-phase7-boundary: PASS: files={len(files)} output={args.output}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        print(f"proof-phase7-boundary: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
