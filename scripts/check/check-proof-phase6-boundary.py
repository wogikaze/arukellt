#!/usr/bin/env python3
"""Independently validate the proof Phase 6 boundary receipt."""
from __future__ import annotations
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
EXPECTED_FILES = {
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
}
EXPECTED_CHECKS = {
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
}
EXPECTED_SCALARS = {
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
        print("usage: check-proof-phase6-boundary.py RECEIPT.json", file=sys.stderr)
        return 2
    validate(json.loads(Path(sys.argv[1]).read_text(encoding="utf-8")))
    print(f"proof-phase6-boundary-check: PASS: files={len(EXPECTED_FILES)}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        print(f"proof-phase6-boundary-check: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
