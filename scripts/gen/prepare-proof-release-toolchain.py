#!/usr/bin/env python3
"""Prepare the exact toolchain and validators trusted by a release proof run."""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

COMPONENTS = (
    ("arukellt-source-proof-binding-v3", "source-artifact-binding", "3", None),
    ("arukellt-source-proof-binding-validator-v3", "source-artifact-binding-validator", "3", "scripts/proof/source_proof_binding.py"),
    ("arukellt-release-provenance-validator-v1", "release-provenance-validator", "1", "scripts/proof/release_provenance.py"),
    ("arukellt-release-payload-validator-v1", "release-payload-validator", "1", "scripts/proof/release_payload.py"),
    ("arukellt-release-gate-v1", "proof-required-release-gate", "1", "scripts/proof/release_gate.py"),
    ("arukellt-release-authorization-v1", "release-authorization-writer", "1", "scripts/proof/release_authorization.py"),
    ("arukellt-proof-release-cli-v1", "release-gate-cli", "1", "scripts/check/check-proof-required-release.py"),
    ("arukellt-proof-trust-validator-v1", "trust-artifact-validator", "1", "scripts/proof/trust.py"),
    ("arukellt-solver-receipt-generator-v1", "proof-receipt-generator", "1", "scripts/proof/solver_receipts.py"),
    ("arukellt-solver-process-driver-v1", "solver-process-driver", "1", "scripts/proof/solver_driver.py"),
    ("arukellt-proof-solver-cli-v1", "solver-driver-cli", "1", "scripts/run/run-proof-solver.py"),
    ("arukellt-verified-core-smtlib-v1", "smt-translator-implementation", "1", "scripts/proof/smtlib_v1.py"),
    ("arukellt-typed-corehir-validator-v1", "typed-corehir-validator", "1", "scripts/proof/typed_corehir.py"),
    ("arukellt-typed-corehir-validator-impl-v1", "typed-corehir-validator-implementation", "1", "scripts/proof/typed_corehir_impl.py"),
    ("arukellt-proof-common-v1", "proof-validation-library", "1", "scripts/proof/common.py"),
    ("arukellt-typed-corehir-contract-converter-v2", "typed-proof-boundary", "2", "scripts/proof/typed_corehir_contract_convert.py"),
    ("arukellt-typed-corehir-expression-converter-v1", "typed-proof-expression-library", "1", "scripts/proof/typed_corehir_convert.py"),
    ("arukellt-source-contract-profile-normalizer-v1", "semantic-profile-normalizer", "1", "scripts/proof/normalize_source_contract_profile.py"),
    ("arukellt-verified-core-v1", "artifact-validator", "1", "scripts/proof/verified_core.py"),
)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--runtime", type=Path, required=True)
    parser.add_argument("--source-binding", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--toolchain-output", type=Path, required=True)
    parser.add_argument("--z3", type=Path, required=True)
    args = parser.parse_args()

    for label, path in (
        ("runtime", args.runtime),
        ("source binding", args.source_binding),
        ("z3", args.z3),
    ):
        if not path.is_file():
            raise ValueError(f"{label} missing: {path}")
    output = args.output_dir
    output.mkdir(parents=True, exist_ok=True)
    runtime_name = "arukellt-s2-runtime.wasm"
    z3_name = "z3"
    binding_name = "source-proof-binding.json"
    shutil.copy2(args.runtime, output / runtime_name)
    shutil.copy2(args.z3, output / z3_name)
    shutil.copy2(args.source_binding, output / binding_name)

    trusted_components: list[dict[str, str]] = []
    for name, role, version, relative in COMPONENTS:
        if relative is None:
            artifact_name = binding_name
        else:
            source = ROOT / relative
            if not source.is_file():
                raise ValueError(f"trusted component missing: {relative}")
            artifact_name = source.name
            destination = output / artifact_name
            if destination.exists() and destination.read_bytes() != source.read_bytes():
                raise ValueError(f"trusted component basename collision: {relative}")
            shutil.copy2(source, destination)
        trusted_components.append(
            {
                "name": name,
                "role": role,
                "version": version,
                "artifact": artifact_name,
            }
        )

    version = subprocess.check_output([str(args.z3), "--version"], text=True).strip()
    document = {
        "schema": "arukellt-proof-toolchain",
        "schema_version": 1,
        "producer": {
            "name": "arukellt-selfhost-typed-contract-emitter",
            "version": "current-s2-runtime",
            "executable": runtime_name,
            "arguments": [
                "compile",
                "tests/verified-core/contract_identity.ark",
                "--emit",
                "typed-corehir",
            ],
        },
        "translator": {
            "name": "arukellt-verified-core-smtlib",
            "version": "1",
            "executable": "smtlib_v1.py",
        },
        "solver": {
            "name": "z3",
            "version": version,
            "executable": z3_name,
            "arguments": ["-in", "-smt2"],
        },
        "semantic_profile": {
            "integer_model": "mathematical",
            "overflow": "checked",
            "floating_point": "unsupported",
            "memory": "pure-values",
        },
        "assumptions": [],
        "trusted_components": trusted_components,
        "limits": {
            "timeout_ms": 10000,
            "memory_bytes": 1073741824,
        },
    }
    args.toolchain_output.parent.mkdir(parents=True, exist_ok=True)
    args.toolchain_output.write_text(
        json.dumps(document, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(
        "proof-release-toolchain: PASS: "
        f"components={len(trusted_components)} output={args.toolchain_output}"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, subprocess.SubprocessError) as exc:
        print(f"proof-release-toolchain: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
