#!/usr/bin/env python3
"""Post-process typed VerifiedCore boundary evidence for proof phases 0-3."""

from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path

import write_typed_verified_core_boundary_receipt_impl as _impl

ROOT = Path(__file__).resolve().parents[2]
EXTRA_FILES = (
    "scripts/gen/write_typed_verified_core_boundary_receipt_impl.py",
    "scripts/proof/verified_core_typed_impl.py",
    "scripts/check/check-proof-phase3-modular.py",
)


def _output_path() -> Path:
    if "--output" in sys.argv:
        index = sys.argv.index("--output")
        return Path(sys.argv[index + 1])
    return ROOT / ".build" / "proof" / "typed-verified-core-boundary.json"


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    digest.update(path.read_bytes())
    return digest.hexdigest()


def main() -> int:
    result = _impl.main()
    if result != 0:
        return result
    output = _output_path()
    document = json.loads(output.read_text(encoding="utf-8"))
    document["semantic_checks"] = [
        "operator-arity-and-TypeId-preservation",
        "contract-root-typing",
        "result-return-TypeId-equality",
        "parameter-signature-local-bijection",
        "constant-payload-typing",
        "contract-kind-expression-id-uniqueness",
        "straight-line-instruction-typing",
        "acyclic-cfg-edge-typing",
        "direct-call-contract-typing",
        "recursive-call-rejection",
        "semantic-admission-before-SMT",
    ]
    present = {entry["path"] for entry in document["files"]}
    for relative in EXTRA_FILES:
        if relative in present:
            continue
        path = ROOT / relative
        if not path.is_file():
            raise ValueError(f"typed VerifiedCore boundary file missing: {relative}")
        document["files"].append({"path": relative, "sha256": _sha256(path)})
    output.write_text(json.dumps(document, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"typed-verified-core-boundary-receipt-v3: PASS: files={len(document['files'])}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        print(f"typed-verified-core-boundary-receipt: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
