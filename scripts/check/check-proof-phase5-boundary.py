#!/usr/bin/env python3
"""Independently validate the proof Phase 5 hash-boundary receipt."""
from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
EXPECTED_FILES = {
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
}
EXPECTED_CHECKS = {
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
}
EXPECTED_SCALARS = {
    "schema": "arukellt-proof-phase-boundary",
    "schema_version": 1,
    "phase": 5,
    "status": "enforced",
    "capability_profile": "proof-phases-0-5@2",
    "source_schema": "arukellt-typed-corehir@2",
    "target_schema": "arukellt-verified-core@1",
    "converter": "arukellt-typed-corehir-converter-v5",
    "semantic_validator": "typed_admission_v5_final.py@5",
    "solver_adapter": "smtlib_typed_v5_final.py@5",
    "aggregate_encoding": "arukellt-smt-datatype-v1",
    "failure_action": "reject-before-SMT-generation",
}


def _fail(message: str) -> None:
    raise ValueError(message)


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    digest.update(path.read_bytes())
    return digest.hexdigest()


def validate(value: object) -> None:
    if not isinstance(value, dict):
        _fail("receipt must be an object")
    expected_keys = set(EXPECTED_SCALARS) | {"semantic_checks", "files"}
    if set(value) != expected_keys:
        _fail(f"top-level key set mismatch: {sorted(set(value) ^ expected_keys)}")
    for key, expected in EXPECTED_SCALARS.items():
        if value.get(key) != expected:
            _fail(f"{key} mismatch")
    checks = value.get("semantic_checks")
    if not isinstance(checks, list) or len(checks) != len(set(checks)) or set(checks) != EXPECTED_CHECKS:
        _fail("semantic_checks set mismatch")
    files = value.get("files")
    if not isinstance(files, list):
        _fail("files must be an array")
    by_path: dict[str, str] = {}
    for index, entry in enumerate(files):
        if not isinstance(entry, dict) or set(entry) != {"path", "sha256"}:
            _fail(f"files[{index}] must contain exactly path/sha256")
        path = entry.get("path")
        digest = entry.get("sha256")
        if not isinstance(path, str) or not path or path.startswith("/") or ".." in Path(path).parts:
            _fail(f"files[{index}].path invalid")
        if path in by_path:
            _fail(f"duplicate boundary path: {path}")
        if not isinstance(digest, str) or len(digest) != 64 or any(ch not in "0123456789abcdef" for ch in digest):
            _fail(f"files[{index}].sha256 invalid")
        by_path[path] = digest
    if set(by_path) != EXPECTED_FILES:
        _fail(f"file set mismatch missing={sorted(EXPECTED_FILES-set(by_path))} extra={sorted(set(by_path)-EXPECTED_FILES)}")
    for relative in sorted(EXPECTED_FILES):
        path = ROOT / relative
        if not path.is_file():
            _fail(f"boundary file missing: {relative}")
        actual = _sha256(path)
        if by_path[relative] != actual:
            _fail(f"sha256 mismatch: {relative}")


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: check-proof-phase5-boundary.py RECEIPT.json", file=sys.stderr)
        return 2
    path = Path(sys.argv[1])
    value = json.loads(path.read_text(encoding="utf-8"))
    validate(value)
    print(f"proof-phase5-boundary-check: PASS: files={len(EXPECTED_FILES)}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        print(f"proof-phase5-boundary-check: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
