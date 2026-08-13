#!/usr/bin/env python3
"""Write hash-bound evidence for the typed VerifiedCore admission boundary."""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BOUNDARY_FILES = (
    "docs/data/proof-capabilities-v1.json",
    "scripts/proof/capabilities.py",
    "scripts/gen/convert-typed-corehir.py",
    "scripts/gen/write-smt-vcs.py",
    "scripts/gen/write-typed-verified-core-boundary-receipt.py",
    "scripts/proof/typed_corehir_typed_convert.py",
    "scripts/proof/typed_corehir_program_convert.py",
    "scripts/proof/verified_core.py",
    "scripts/proof/verified_core_program.py",
    "scripts/proof/verified_core_typed.py",
    "scripts/proof/typed_verified_core_receipt.py",
    "scripts/proof/smtlib_typed_v1.py",
    "scripts/proof/smtlib_v1.py",
    "scripts/check/check-typed-verified-core.py",
    "scripts/check/check-typed-verified-core-boundary.py",
    "scripts/check/check-typed-verified-core-boundary-receipt.py",
    "scripts/tests/test_typed_corehir_convert.py",
    "scripts/tests/test_verified_core_typed.py",
    "scripts/tests/test_smtlib_typed_v1.py",
    "scripts/tests/test_proof_program_phases.py",
    "scripts/tests/test_proof_phase3_calls.py",
    "scripts/tests/test_typed_verified_core_receipt.py",
    "tests/proof/typed-corehir.json",
    ".github/workflows/typed-corehir-proof-pipeline.yml",
    "release/proof-policy.json",
)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    digest.update(path.read_bytes())
    return digest.hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=ROOT / ".build" / "proof" / "typed-verified-core-boundary.json")
    args = parser.parse_args()
    files = []
    for relative in BOUNDARY_FILES:
        path = ROOT / relative
        if not path.is_file():
            raise ValueError(f"typed VerifiedCore boundary file missing: {relative}")
        files.append({"path": relative, "sha256": sha256(path)})
    document = {
        "schema": "arukellt-typed-verified-core-boundary",
        "schema_version": 1,
        "status": "enforced",
        "source_schema": "arukellt-typed-corehir@1",
        "target_schema": "arukellt-verified-core@1",
        "converter": "arukellt-typed-corehir-converter-v3",
        "logical_integer_metadata": "explicit-bits-and-signedness",
        "type_name_semantics": "identity-only",
        "structural_validator": "verified_core.py@1",
        "semantic_validator": "verified_core_typed.py@2",
        "solver_adapter": "smtlib_typed_v1.py@1",
        "semantic_checks": [
            "operator-arity-and-TypeId-preservation",
            "contract-root-typing",
            "result-return-TypeId-equality",
            "parameter-signature-local-bijection",
            "constant-payload-typing",
            "global-contract-expression-id-uniqueness",
            "straight-line-instruction-typing",
            "acyclic-cfg-edge-typing",
            "direct-call-contract-typing",
            "recursive-call-rejection",
            "semantic-admission-before-SMT",
        ],
        "failure_action": "reject-before-SMT-generation",
        "files": files,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(document, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"typed-verified-core-boundary-receipt: PASS: files={len(files)} output={args.output}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError) as exc:
        print(f"typed-verified-core-boundary-receipt: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
