#!/usr/bin/env python3
"""Write the hash-bound proof Phase 6 machine-integer boundary receipt."""
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
    "scripts/gen/write-proof-phase5-boundary-receipt.py",
    "scripts/check/check-proof-phase5-boundary.py",
    "scripts/proof/typed_corehir_v1_scalar_v3.py",
    "scripts/gen/upgrade-typed-corehir-v1-scalar-v3.py",
    "scripts/tests/test_typed_corehir_v1_scalar_v3.py",
    "scripts/proof/machine_integer_semantics.py",
    "scripts/proof/typed_admission_v6.py",
    "scripts/proof/proof_phase6_vc.py",
    "scripts/proof/smtlib_typed_v6.py",
    "scripts/tests/test_proof_phase6_machine_int.py",
    "scripts/gen/write-proof-phase6-boundary-receipt.py",
    "scripts/check/check-proof-phase6-boundary.py",
)
CHECKS = [
    "phase5-boundary-hash-chain",
    "selfhost-v1-scalar-upgrade-fail-closed",
    "raw-to-canonical-v3-machine-profile",
    "overflow-capable-source-upgrade-rejection",
    "signed-i32-i64-TypeId-metadata",
    "display-name-independent-machine-semantics",
    "checked-add-sub-mul-neg-overflow-vc",
    "signed-div-zero-vc",
    "signed-div-min-overflow-vc",
    "signed-div-rem-truncation-toward-zero",
    "constant-shift-count-width-masking",
    "symbolic-shift-rejection",
    "signed-widen-typing",
    "checked-narrow-range-vc",
    "machine-parameter-result-range-vc",
    "out-of-range-constant-rejection",
    "semantic-admission-before-SMT",
]


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    digest.update(path.read_bytes())
    return digest.hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, default=ROOT / ".build/proof/proof-phase6-boundary.json")
    args = parser.parse_args()
    files = []
    for relative in FILES:
        path = ROOT / relative
        if not path.is_file():
            raise ValueError(f"Phase 6 boundary file missing: {relative}")
        files.append({"path": relative, "sha256": sha256(path)})
    value = {
        "schema": "arukellt-proof-phase-boundary",
        "schema_version": 1,
        "phase": 6,
        "status": "enforced",
        "capability_profile": "proof-phases-0-7@3",
        "source_schema": "arukellt-typed-corehir@3",
        "source_upgrade": "arukellt-selfhost-v1-scalar-upgrade-v1",
        "target_schema": "arukellt-verified-core@1",
        "semantic_validator": "typed_admission_v6.py@6",
        "solver_adapter": "smtlib_typed_v6.py@6",
        "integer_profile": "arukellt-machine-int-v1",
        "integer_encoding": "arukellt-machine-int-range-v1",
        "semantic_checks": CHECKS,
        "failure_action": "reject-before-SMT-generation",
        "files": files,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"proof-phase6-boundary: PASS: files={len(files)} output={args.output}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        print(f"proof-phase6-boundary: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
