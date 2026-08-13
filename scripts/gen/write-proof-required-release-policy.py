#!/usr/bin/env python3
"""Write the canonical proof-required release policy beside its proof bundle."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--label", default="source-contract-identity")
    args = parser.parse_args()
    document = {
        "schema": "arukellt-proof-release-policy",
        "schema_version": 1,
        "mode": "proof-required",
        "hard_gates": {
            "versioned_boundary_artifacts": True,
            "explicit_backend_type_abi_layout": True,
            "typed_verified_core_emission": True,
            "optimizer_translation_validation": True,
            "solver_trust_manifest": True,
            "legacy_mutable_table_removed": True,
            "proof_receipt_release_enforced": True,
        },
        "artifacts": [
            {
                "label": args.label,
                "subject": "verified-core.json",
                "trust_manifest": "trust-manifest.json",
                "receipt": "proof-receipt.json",
                "solver_output": "solver-output.txt",
                "translation_receipts": [],
            }
        ],
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(document, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(f"proof-required-release-policy: PASS: output={args.output}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except OSError as exc:
        print(f"proof-required-release-policy: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
