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

from proof.release_gate import (  # noqa: E402
    ProofRequiredReleaseError,
    validate_proof_required_release,
)
from proof.source_proof_binding import SourceProofBindingError  # noqa: E402
from proof.common import ValidationError  # noqa: E402


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
    parser.add_argument("--backend-typeid-audit", type=Path, required=True)
    parser.add_argument("--optimizer-translation-registry", type=Path, required=True)
    parser.add_argument("--corehir-body-boundary-validator", type=Path, required=True)
    args = parser.parse_args()

    paths = {
        "source": args.source,
        "producer_executable": args.producer_executable,
        "typed_corehir": args.typed_corehir,
        "verified_core_machine": args.verified_core_machine,
        "verified_core_normalized": args.verified_core_normalized,
        "solver_input": args.solver_input,
        "backend_typeid_audit": args.backend_typeid_audit,
        "optimizer_translation_registry": args.optimizer_translation_registry,
        "corehir_body_boundary_validator": args.corehir_body_boundary_validator,
    }
    try:
        mode, count = validate_proof_required_release(
            args.policy,
            args.source_binding,
            paths,
        )
    except (
        OSError,
        ValueError,
        ValidationError,
        SourceProofBindingError,
        ProofRequiredReleaseError,
    ) as exc:
        print(f"proof-required-release: FAIL: {exc}", file=sys.stderr)
        return 1
    print(
        "proof-required-release: PASS: "
        f"mode={mode} artifacts={count} source_and_architecture_binding=verified"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
