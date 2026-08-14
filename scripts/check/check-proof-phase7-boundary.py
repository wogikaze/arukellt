#!/usr/bin/env python3
"""Independently validate the proof Phase 7 boundary receipt."""
from __future__ import annotations
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
EXPECTED_FILES = {
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
}
EXPECTED_CHECKS = {
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
}
EXPECTED_SCALARS = {
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
    "failure_action": "reject-before-SMT-generation",
}


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    digest.update(path.read_bytes())
    return digest.hexdigest()


def _fail(message: str) -> None:
    raise ValueError(message)


def validate(value: object) -> None:
    if not isinstance(value, dict):
        _fail("receipt must be an object")
    expected_keys = set(EXPECTED_SCALARS) | {"semantic_checks", "files"}
    if set(value) != expected_keys:
        _fail("top-level key set mismatch")
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
            _fail(f"files[{index}] invalid")
        path = entry.get("path")
        digest = entry.get("sha256")
        if not isinstance(path, str) or not path or path.startswith("/") or ".." in Path(path).parts:
            _fail(f"files[{index}].path invalid")
        if path in by_path:
            _fail(f"duplicate path: {path}")
        if not isinstance(digest, str) or len(digest) != 64 or any(ch not in "0123456789abcdef" for ch in digest):
            _fail(f"files[{index}].sha256 invalid")
        by_path[path] = digest
    if set(by_path) != EXPECTED_FILES:
        _fail(f"file set mismatch missing={sorted(EXPECTED_FILES-set(by_path))} extra={sorted(set(by_path)-EXPECTED_FILES)}")
    for relative in sorted(EXPECTED_FILES):
        path = ROOT / relative
        if not path.is_file():
            _fail(f"boundary file missing: {relative}")
        if by_path[relative] != _sha256(path):
            _fail(f"sha256 mismatch: {relative}")


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: check-proof-phase7-boundary.py RECEIPT.json", file=sys.stderr)
        return 2
    validate(json.loads(Path(sys.argv[1]).read_text(encoding="utf-8")))
    print(f"proof-phase7-boundary-check: PASS: files={len(EXPECTED_FILES)}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        print(f"proof-phase7-boundary-check: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
