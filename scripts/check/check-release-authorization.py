#!/usr/bin/env python3
"""Verify release authorization before any external publish action."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from proof.release_authorization import (  # noqa: E402
    ReleaseAuthorizationError,
    validate_bound_release_authorization,
)
from proof.release_payload import (  # noqa: E402
    ReleasePayloadError,
    validate_release_payload_manifest,
)
from proof.common import load_json  # noqa: E402


def parse_payload(raw: str) -> tuple[str, Path]:
    name, separator, path = raw.partition("=")
    if not separator or not name or not path:
        raise ValueError("--release-payload must be NAME=PATH")
    return name, Path(path)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--authorization", type=Path, required=True)
    parser.add_argument("--repository", required=True)
    parser.add_argument("--commit", required=True)
    parser.add_argument("--tag", required=True)
    parser.add_argument("--policy", type=Path, required=True)
    parser.add_argument("--source-binding", type=Path, required=True)
    parser.add_argument("--trust-manifest", type=Path, required=True)
    parser.add_argument("--proof-receipt", type=Path, required=True)
    parser.add_argument("--release-payload-manifest", type=Path, required=True)
    parser.add_argument("--release-payload", action="append", required=True)
    args = parser.parse_args()

    payloads: dict[str, Path] = {}
    for raw in args.release_payload:
        name, path = parse_payload(raw)
        if name in payloads:
            raise ValueError(f"duplicate release payload name: {name}")
        payloads[name] = path
    try:
        validate_release_payload_manifest(
            load_json(args.release_payload_manifest),
            payloads,
        )
        validate_bound_release_authorization(
            args.authorization,
            repository=args.repository,
            commit_sha=args.commit,
            tag=args.tag,
            policy_path=args.policy,
            source_binding_path=args.source_binding,
            trust_manifest_path=args.trust_manifest,
            proof_receipt_path=args.proof_receipt,
            payload_manifest_path=args.release_payload_manifest,
        )
    except (OSError, ValueError, ReleaseAuthorizationError, ReleasePayloadError) as exc:
        print(f"release-authorization: FAIL: {exc}", file=sys.stderr)
        return 1
    print(
        "release-authorization: PASS: "
        f"repository={args.repository} tag={args.tag} payloads={len(payloads)}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
