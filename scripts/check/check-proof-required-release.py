#!/usr/bin/env python3
"""Validate the complete source-to-ProofReceipt release chain."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from proof.common import ValidationError  # noqa: E402
from proof.release_authorization import (  # noqa: E402
    ReleaseAuthorizationError,
    create_release_authorization,
    write_release_authorization,
)
from proof.release_gate import (  # noqa: E402
    ProofRequiredReleaseError,
    primary_release_artifact_paths,
    validate_proof_required_release,
)
from proof.release_payload import ReleasePayloadError  # noqa: E402
from proof.release_provenance import ReleaseProvenanceError  # noqa: E402
from proof.source_proof_binding import SourceProofBindingError  # noqa: E402


def parse_payload(raw: str) -> tuple[str, Path]:
    name, separator, path = raw.partition("=")
    if not separator or not name or not path:
        raise ValueError("--release-payload must be NAME=PATH")
    return name, Path(path)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("policy", type=Path)
    parser.add_argument("--source-binding", type=Path, required=True)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--producer-executable", type=Path, required=True)
    parser.add_argument("--typed-corehir", type=Path, required=True)
    parser.add_argument("--verified-core-machine", type=Path, required=True)
    parser.add_argument("--verified-core-normalized", type=Path, required=True)
    parser.add_argument("--solver-input", type=Path, required=True)
    parser.add_argument(
        "--backend-typeid-layout-receipt",
        type=Path,
        default=ROOT / ".build" / "proof" / "backend-typeid-layout.json",
    )
    parser.add_argument(
        "--optimizer-translation-registry",
        type=Path,
        default=ROOT / ".build" / "proof" / "mir-opt-translation-registry.json",
    )
    parser.add_argument(
        "--corehir-body-boundary-receipt",
        type=Path,
        default=ROOT / ".build" / "proof" / "corehir-body-boundary.json",
    )
    parser.add_argument("--release-provenance", type=Path, required=True)
    parser.add_argument("--release-payload-manifest", type=Path, required=True)
    parser.add_argument("--release-payload", action="append", required=True)
    parser.add_argument("--expected-repository", required=True)
    parser.add_argument("--expected-commit", required=True)
    parser.add_argument("--expected-tag", required=True)
    parser.add_argument("--authorization-output", type=Path, required=True)
    args = parser.parse_args()

    release_payloads: dict[str, Path] = {}
    for raw in args.release_payload:
        name, path = parse_payload(raw)
        if name in release_payloads:
            raise ValueError(f"duplicate release payload name: {name}")
        release_payloads[name] = path

    paths = {
        "source": args.source,
        "producer_executable": args.producer_executable,
        "typed_corehir": args.typed_corehir,
        "verified_core_machine": args.verified_core_machine,
        "verified_core_normalized": args.verified_core_normalized,
        "solver_input": args.solver_input,
        "backend_typeid_layout_receipt": args.backend_typeid_layout_receipt,
        "optimizer_translation_registry": args.optimizer_translation_registry,
        "corehir_body_boundary_receipt": args.corehir_body_boundary_receipt,
        "release_provenance": args.release_provenance,
        "release_payload_manifest": args.release_payload_manifest,
    }
    try:
        mode, count = validate_proof_required_release(
            args.policy,
            args.source_binding,
            paths,
            expected_repository=args.expected_repository,
            expected_commit=args.expected_commit,
            expected_tag=args.expected_tag,
            release_payloads=release_payloads,
        )
        artifact_paths = primary_release_artifact_paths(args.policy)
        authorization = create_release_authorization(
            repository=args.expected_repository,
            commit_sha=args.expected_commit,
            tag=args.expected_tag,
            policy_path=args.policy,
            source_binding_path=args.source_binding,
            trust_manifest_path=artifact_paths["trust_manifest"],
            proof_receipt_path=artifact_paths["receipt"],
            payload_manifest_path=args.release_payload_manifest,
        )
        write_release_authorization(authorization, args.authorization_output)
    except (
        OSError,
        ValueError,
        ValidationError,
        SourceProofBindingError,
        ReleasePayloadError,
        ReleaseProvenanceError,
        ReleaseAuthorizationError,
        ProofRequiredReleaseError,
    ) as exc:
        print(f"proof-required-release: FAIL: {exc}", file=sys.stderr)
        return 1
    print(
        "proof-required-release: PASS: "
        f"mode={mode} artifacts={count} tag={args.expected_tag} "
        f"authorization={args.authorization_output}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
