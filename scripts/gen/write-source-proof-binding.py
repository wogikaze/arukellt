#!/usr/bin/env python3
"""Write a digest binding for source proof, architecture evidence, and release payload."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from proof.source_proof_binding import (  # noqa: E402
    SourceProofBindingError,
    write_binding,
)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", required=True, type=Path)
    parser.add_argument("--producer-executable", required=True, type=Path)
    parser.add_argument("--typed-corehir", required=True, type=Path)
    parser.add_argument("--verified-core-machine", required=True, type=Path)
    parser.add_argument("--verified-core-normalized", required=True, type=Path)
    parser.add_argument("--solver-input", required=True, type=Path)
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
    parser.add_argument("--release-provenance", required=True, type=Path)
    parser.add_argument("--release-payload-manifest", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    write_binding(
        {
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
        },
        args.output,
    )
    print(f"write-source-proof-binding: PASS: output={args.output}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, SourceProofBindingError, ValueError) as exc:
        print(f"write-source-proof-binding: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
