#!/usr/bin/env python3
"""Write a hash-bound receipt for proof-required release enforcement."""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BOUNDARY_FILES = (
    ".github/workflows/proof-required-release.yml",
    ".github/workflows/versioned-boundary-registry.yml",
    "release/boundary-registry.json",
    "scripts/run/proof-required-release.sh",
    "scripts/check/check-boundary-registry.py",
    "scripts/check/check-boundary-registry-receipt.py",
    "scripts/check/check-versioned-boundary-enforcement.py",
    "scripts/check/check-proof-release-entrypoints.py",
    "scripts/check/check-proof-required-release.py",
    "scripts/check/check-release-authorization.py",
    "scripts/proof/boundary_registry.py",
    "scripts/proof/boundary_registry_receipt.py",
    "scripts/proof/release_gate.py",
    "scripts/proof/release_provenance.py",
    "scripts/proof/release_payload.py",
    "scripts/proof/release_authorization.py",
    "scripts/proof/source_proof_binding.py",
    "scripts/gen/write-release-provenance.py",
    "scripts/gen/write-release-payload-manifest.py",
    "scripts/gen/write-source-proof-binding.py",
    "scripts/gen/prepare-proof-release-toolchain.py",
    "scripts/gen/write-proof-required-release-policy.py",
    "scripts/tests/test_boundary_registry.py",
    "scripts/tests/test_source_proof_binding.py",
    "scripts/tests/test_proof_required_release_gate.py",
    "scripts/tests/test_release_authorization.py",
    "release/proof-policy.json",
)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    digest.update(path.read_bytes())
    return digest.hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--output",
        type=Path,
        default=ROOT / ".build" / "proof" / "proof-release-enforcement.json",
    )
    args = parser.parse_args()

    files: list[dict[str, str]] = []
    for relative in BOUNDARY_FILES:
        path = ROOT / relative
        if not path.is_file():
            raise ValueError(f"release enforcement file missing: {relative}")
        files.append({"path": relative, "sha256": sha256(path)})
    document = {
        "schema": "arukellt-proof-release-enforcement",
        "schema_version": 1,
        "status": "enforced",
        "tag_pattern": "refs/tags/v*",
        "authorization_required": True,
        "authorization_binds": [
            "repository",
            "commit",
            "tag",
            "proof policy",
            "source and architecture binding",
            "pinned major boundary registry and validation receipt",
            "TrustManifest",
            "ProofReceipt",
            "release payload manifest",
        ],
        "receipt_replay": "rejected",
        "payload_substitution": "rejected",
        "unguarded_publish_entrypoints": "rejected",
        "files": files,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(document, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(
        "proof-release-enforcement-receipt: PASS: "
        f"files={len(files)} output={args.output}"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError) as exc:
        print(f"proof-release-enforcement-receipt: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
