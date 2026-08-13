#!/usr/bin/env python3
"""Independently revalidate a major-boundary registry receipt."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from proof.boundary_registry import BoundaryRegistryError, GitHubRawSource, load_registry  # noqa: E402
from proof.boundary_registry_receipt import (  # noqa: E402
    BoundaryRegistryReceiptError,
    load_validation_receipt,
    validate_validation_receipt,
)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--registry",
        type=Path,
        default=ROOT / "release" / "boundary-registry.json",
    )
    parser.add_argument(
        "--receipt",
        type=Path,
        default=ROOT / ".build" / "proof" / "boundary-registry-validation.json",
    )
    parser.add_argument("--repository")
    parser.add_argument("--timeout-seconds", type=float, default=20.0)
    args = parser.parse_args()

    registry_path = args.registry.resolve()
    registry = load_registry(registry_path)
    if args.repository is not None and args.repository != registry["repository"]:
        raise BoundaryRegistryReceiptError(
            f"repository mismatch: registry={registry['repository']} requested={args.repository}"
        )
    receipt = load_validation_receipt(args.receipt.resolve())
    validated = validate_validation_receipt(
        receipt,
        registry_path=registry_path,
        source=GitHubRawSource(timeout_seconds=args.timeout_seconds),
    )
    print(
        "boundary-registry-receipt: PASS: "
        f"boundaries={validated['boundary_count']} files={len(validated['files'])}"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, BoundaryRegistryError, BoundaryRegistryReceiptError) as exc:
        print(f"boundary-registry-receipt: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
