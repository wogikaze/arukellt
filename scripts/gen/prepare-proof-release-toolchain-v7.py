#!/usr/bin/env python3
"""Prepare the proof-required release toolchain through Phase 7."""
from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from proof.typed_corehir_v3 import validate_document as validate_v3_source  # noqa: E402

BASE = ROOT / "scripts" / "gen" / "prepare-proof-release-toolchain-v5.py"
EXTRA_COMPONENTS = (
    ("arukellt-proof-capability-profile-v3", "proof-capability-profile", "3", "docs/data/proof-capabilities-v3.json"),
    ("arukellt-proof-capability-validator-v3", "proof-capability-validator", "3", "scripts/proof/capabilities_v3.py"),
    ("arukellt-typed-corehir-v3-schema", "proof-source-schema", "3", "schemas/typed-corehir-v3.schema.json"),
    ("arukellt-typed-corehir-v3-validator", "proof-source-validator", "3", "scripts/proof/typed_corehir_v3.py"),
    ("arukellt-typed-corehir-v7-converter", "proof-source-converter", "7", "scripts/proof/typed_corehir_v3_convert.py"),
    ("arukellt-machine-int-v1", "machine-integer-semantic-validator", "1", "scripts/proof/machine_integer_semantics.py"),
    ("arukellt-machine-int-range-v1", "machine-integer-vc-renderer", "1", "scripts/proof/proof_phase6_vc.py"),
    ("arukellt-typed-proof-admission-v6", "phase6-typed-artifact-validator", "6", "scripts/proof/typed_admission_v6.py"),
    ("arukellt-proof-phase6-boundary-v1", "phase6-boundary-checker", "1", "scripts/check/check-proof-phase6-boundary.py"),
    ("arukellt-readonly-heap-v1", "readonly-memory-semantic-validator", "1", "scripts/proof/readonly_memory_semantics.py"),
    ("arukellt-readonly-heap-compat-v7", "readonly-memory-legacy-adapter", "7", "scripts/proof/readonly_memory_compat_v7.py"),
    ("arukellt-readonly-heap-limits-v7", "readonly-memory-composition-validator", "7", "scripts/proof/readonly_memory_phase7_limits.py"),
    ("arukellt-typed-proof-admission-v7", "typed-artifact-validator", "7", "scripts/proof/typed_admission_v7.py"),
    ("arukellt-readonly-heap-smt-v1", "readonly-memory-vc-renderer", "1", "scripts/proof/proof_phase7_vc.py"),
    ("arukellt-phase7-smt-adapter", "typed-smt-adapter", "7", "scripts/proof/smtlib_typed_v7.py"),
    ("arukellt-proof-phase7-boundary-v1", "phase7-boundary-checker", "1", "scripts/check/check-proof-phase7-boundary.py"),
)


def _copy_component(document: dict, output_dir: Path, *, name: str, role: str, version: str, source: Path, artifact: str | None = None) -> None:
    if not source.is_file():
        raise ValueError(f"trusted component missing: {source}")
    artifact_name = artifact or source.name
    destination = output_dir / artifact_name
    if destination.exists() and destination.read_bytes() != source.read_bytes():
        raise ValueError(f"trusted component basename collision: {source}")
    shutil.copy2(source, destination)
    document["trusted_components"] = [entry for entry in document["trusted_components"] if entry["role"] != role]
    document["trusted_components"].append({"name": name, "role": role, "version": version, "artifact": artifact_name})


def _source_version(path: Path) -> tuple[int, dict]:
    if not path.is_file():
        raise ValueError(f"TypedCoreHIR source missing: {path}")
    document = json.loads(path.read_text(encoding="utf-8"))
    if document.get("schema") != "arukellt-typed-corehir":
        raise ValueError("TypedCoreHIR source schema mismatch")
    version = document.get("schema_version")
    if type(version) is not int or version not in {1, 2, 3}:
        raise ValueError(f"unsupported TypedCoreHIR source version: {version!r}")
    if version == 3:
        validate_v3_source(document)
    return version, document


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--runtime", type=Path, required=True)
    parser.add_argument("--source-binding", type=Path, required=True)
    parser.add_argument("--typed-corehir", type=Path, required=True)
    parser.add_argument("--phase6-boundary", type=Path, required=True)
    parser.add_argument("--phase7-boundary", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--toolchain-output", type=Path, required=True)
    parser.add_argument("--z3", type=Path, required=True)
    args = parser.parse_args()

    source_version, _ = _source_version(args.typed_corehir)
    command = [
        sys.executable, str(BASE),
        "--runtime", str(args.runtime),
        "--source-binding", str(args.source_binding),
        "--output-dir", str(args.output_dir),
        "--toolchain-output", str(args.toolchain_output),
        "--z3", str(args.z3),
    ]
    subprocess.run(command, cwd=ROOT, check=True)
    document = json.loads(args.toolchain_output.read_text(encoding="utf-8"))
    document["semantic_profile"]["source_schema_version"] = source_version
    document["semantic_profile"]["phase67_available"] = True
    document["semantic_profile"]["phase67_active"] = source_version == 3

    if source_version == 3:
        document["translator"]["version"] = "7"
        document["semantic_profile"].update({
            "integer_model": "machine",
            "overflow": "checked",
            "floating_point": "unsupported",
            "machine_integer_model": "arukellt-machine-int-v1",
            "machine_integer_encoding": "arukellt-machine-int-range-v1",
            "memory": "read-only-heap",
            "memory_model": "arukellt-readonly-heap-v1",
            "memory_encoding": "arukellt-readonly-heap-smt-v1",
            "capability_profile": "proof-phases-0-7@3",
        })
        for name, role, version, relative in EXTRA_COMPONENTS:
            _copy_component(document, args.output_dir, name=name, role=role, version=version, source=ROOT / relative)

    _copy_component(
        document,
        args.output_dir,
        name="arukellt-proof-phase6-boundary-receipt",
        role="phase6-boundary-receipt",
        version="1",
        source=args.phase6_boundary,
        artifact="proof-phase6-boundary.json",
    )
    _copy_component(
        document,
        args.output_dir,
        name="arukellt-proof-phase7-boundary-receipt",
        role="phase7-boundary-receipt",
        version="1",
        source=args.phase7_boundary,
        artifact="proof-phase7-boundary.json",
    )
    args.toolchain_output.write_text(json.dumps(document, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(
        "proof-release-toolchain-v7: PASS: "
        f"source_schema=v{source_version} phase67_active={source_version == 3} "
        f"components={len(document['trusted_components'])} output={args.toolchain_output}"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, subprocess.SubprocessError, json.JSONDecodeError) as exc:
        print(f"proof-release-toolchain-v7: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
