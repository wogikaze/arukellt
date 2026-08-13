#!/usr/bin/env python3
"""Validate every pinned major boundary and write a versioned receipt."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from proof.boundary_registry import (  # noqa: E402
    BoundaryRegistryError,
    GitHubRawSource,
    load_registry,
)
from proof.boundary_registry_receipt import (  # noqa: E402
    BoundaryRegistryReceiptError,
    create_validation_receipt,
    validate_validation_receipt,
    write_validation_receipt,
)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--registry",
        type=Path,
        default=ROOT / "release" / "boundary-registry.json",
    )
    parser.add_argument("--repository")
    parser.add_argument(
        "--receipt-output",
        type=Path,
        default=ROOT / ".build" / "proof" / "boundary-registry-validation.json",
    )
    parser.add_argument("--timeout-seconds", type=float, default=20.0)
    args = parser.parse_args()

    registry_path = args.registry.resolve()
    registry = load_registry(registry_path)
    if args.repository is not None and args.repository != registry["repository"]:
        raise BoundaryRegistryError(
            f"repository mismatch: registry={registry['repository']} requested={args.repository}"
        )
    source = GitHubRawSource(timeout_seconds=args.timeout_seconds)
    receipt = create_validation_receipt(registry_path, source)
    output = args.receipt_output.resolve()
    write_validation_receipt(receipt, output)
    validate_validation_receipt(receipt, registry_path=registry_path)
    print(
        "boundary-registry: PASS: "
        f"boundaries={receipt['boundary_count']} files={len(receipt['files'])} output={output}"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, BoundaryRegistryError, BoundaryRegistryReceiptError) as exc:
        print(f"boundary-registry: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
