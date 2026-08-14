#!/usr/bin/env python3
"""Independently verify the final proof Phase 5 hash-boundary receipt."""
from __future__ import annotations
import hashlib, json, sys
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]
REQUIRED_CHECKS = {
    "exact-callee-interface-sha256-binding", "recursive-call-rejection",
    "annotated-loop-invariant-typing", "loop-initiation-preservation-exit-vc",
    "strict-decreases-termination-vc", "unannotated-cycle-rejection",
    "pure-aggregate-type-metadata", "aggregate-constructor-projection-typing",
    "enum-variant-payload-typing", "deterministic-smt-datatype-v1",
    "recursive-aggregate-rejection", "typed-corehir-v2-aggregate-lowering",
    "semantic-admission-before-SMT",
}

def sha(path: Path) -> str:
    digest = hashlib.sha256(); digest.update(path.read_bytes()); return digest.hexdigest()

def main() -> int:
    if len(sys.argv) != 2: raise ValueError("usage: check-proof-phase5-boundary-receipt.py RECEIPT")
    receipt = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
    if receipt.get("schema") != "arukellt-proof-phase-boundary" or receipt.get("schema_version") != 1: raise ValueError("invalid Phase 5 boundary schema")
    if receipt.get("phase") != 5 or receipt.get("status") != "enforced": raise ValueError("receipt is not enforced Phase 5")
    if receipt.get("capability_profile") != "proof-phases-0-5@2": raise ValueError("wrong capability profile")
    if receipt.get("source_schema") != "arukellt-typed-corehir@2" or receipt.get("converter") != "arukellt-typed-corehir-converter-v5": raise ValueError("wrong source/converter binding")
    if receipt.get("semantic_validator") != "typed_admission_v5_final.py@5": raise ValueError("wrong semantic validator binding")
    if receipt.get("solver_adapter") != "smtlib_typed_v5_final.py@5" or receipt.get("aggregate_encoding") != "arukellt-smt-datatype-v1": raise ValueError("wrong solver encoding binding")
    if set(receipt.get("semantic_checks", [])) != REQUIRED_CHECKS: raise ValueError("Phase 5 semantic check set mismatch")
    seen: set[str] = set()
    files = receipt.get("files")
    if not isinstance(files, list) or not files: raise ValueError("receipt files missing")
    for index, entry in enumerate(files):
        path = entry.get("path")
        if not isinstance(path, str) or not path or path in seen: raise ValueError(f"files[{index}].path: invalid or duplicate")
        seen.add(path); resolved = ROOT / path
        if not resolved.is_file(): raise ValueError(f"bound file missing: {path}")
        if entry.get("sha256") != sha(resolved): raise ValueError(f"bound file digest mismatch: {path}")
    print(f"proof-phase5-boundary-receipt: PASS: files={len(files)}")
    return 0

if __name__ == "__main__":
    try: raise SystemExit(main())
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        print(f"proof-phase5-boundary-receipt: FAIL: {exc}", file=sys.stderr); raise SystemExit(1)
