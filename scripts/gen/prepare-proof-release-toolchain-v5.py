#!/usr/bin/env python3
"""Prepare a release proof toolchain extended with Phase 5 pure aggregate trust components."""
from __future__ import annotations
import argparse, json, shutil, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / "scripts" / "gen" / "prepare-proof-release-toolchain.py"
EXTRA_COMPONENTS = (
    ("arukellt-proof-capability-profile-v2", "proof-capability-profile", "2", "docs/data/proof-capabilities-v2.json"),
    ("arukellt-proof-capability-validator-v2", "proof-capability-validator", "2", "scripts/proof/capabilities_v2.py"),
    ("arukellt-typed-proof-admission-v5", "typed-artifact-validator", "5", "scripts/proof/typed_admission_v5.py"),
    ("arukellt-pure-aggregate-types-v5", "aggregate-type-validator", "5", "scripts/proof/aggregate_types.py"),
    ("arukellt-pure-aggregate-semantics-v5", "aggregate-expression-validator", "5", "scripts/proof/aggregate_semantics.py"),
    ("arukellt-pure-aggregate-instructions-v5", "aggregate-instruction-validator", "5", "scripts/proof/aggregate_instructions.py"),
    ("arukellt-smt-datatype-v1", "aggregate-smt-renderer", "1", "scripts/proof/proof_phase5_vc.py"),
    ("arukellt-proof-phase5-boundary-v1", "phase5-boundary-checker", "1", "scripts/check/check-proof-phase5-boundary.py"),
    ("arukellt-typed-boundary-receipt-v5", "typed-boundary-receipt-validator", "5", "scripts/proof/typed_verified_core_receipt_v5.py"),
)

def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--runtime", type=Path, required=True)
    parser.add_argument("--source-binding", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--toolchain-output", type=Path, required=True)
    parser.add_argument("--z3", type=Path, required=True)
    args = parser.parse_args()
    command = [sys.executable, str(BASE), "--runtime", str(args.runtime), "--source-binding", str(args.source_binding), "--output-dir", str(args.output_dir), "--toolchain-output", str(args.toolchain_output), "--z3", str(args.z3)]
    subprocess.run(command, cwd=ROOT, check=True)
    document = json.loads(args.toolchain_output.read_text(encoding="utf-8"))
    document["translator"]["version"] = "5"
    document["semantic_profile"]["aggregate_model"] = "pure-algebraic-datatypes"
    document["semantic_profile"]["aggregate_encoding"] = "arukellt-smt-datatype-v1"
    existing_roles = {entry["role"] for entry in document["trusted_components"]}
    for name, role, version, relative in EXTRA_COMPONENTS:
        source = ROOT / relative
        if not source.is_file(): raise ValueError(f"trusted component missing: {relative}")
        artifact = source.name
        destination = args.output_dir / artifact
        if destination.exists() and destination.read_bytes() != source.read_bytes(): raise ValueError(f"trusted component basename collision: {relative}")
        shutil.copy2(source, destination)
        document["trusted_components"] = [entry for entry in document["trusted_components"] if entry["role"] != role]
        document["trusted_components"].append({"name": name, "role": role, "version": version, "artifact": artifact})
        existing_roles.add(role)
    args.toolchain_output.write_text(json.dumps(document, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"proof-release-toolchain-v5: PASS: components={len(document['trusted_components'])} output={args.toolchain_output}")
    return 0

if __name__ == "__main__":
    try: raise SystemExit(main())
    except (OSError, ValueError, subprocess.SubprocessError, json.JSONDecodeError) as exc:
        print(f"proof-release-toolchain-v5: FAIL: {exc}", file=sys.stderr); raise SystemExit(1)
