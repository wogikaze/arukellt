#!/usr/bin/env python3
"""Prepare the exact Phase 5 proof toolchain trusted by aggregate-capable release proof runs."""
from __future__ import annotations
import argparse, json, shutil, subprocess, sys
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / "scripts" / "gen" / "prepare-proof-release-toolchain.py"
COMPONENTS = (
    ("arukellt-proof-capability-profile-v2", "proof-capability-profile", "2", "docs/data/proof-capabilities-v2.json"),
    ("arukellt-proof-capability-validator-v2", "proof-capability-validator", "2", "scripts/proof/capabilities_v2.py"),
    ("arukellt-typed-corehir-v2-validator", "typed-corehir-validator", "2", "scripts/proof/typed_corehir_v2.py"),
    ("arukellt-typed-corehir-converter-v5", "typed-proof-boundary", "5", "scripts/proof/typed_corehir_v2_convert_final.py"),
    ("arukellt-typed-admission-v5", "typed-artifact-validator", "5", "scripts/proof/typed_admission_v5_final.py"),
    ("arukellt-pure-aggregate-types-v5", "aggregate-type-validator", "5", "scripts/proof/aggregate_types.py"),
    ("arukellt-pure-aggregate-semantics-v5", "aggregate-expression-validator", "5", "scripts/proof/aggregate_semantics.py"),
    ("arukellt-pure-aggregate-instructions-v5", "aggregate-instruction-validator", "5", "scripts/proof/aggregate_instructions.py"),
    ("arukellt-aggregate-legacy-view-v5", "aggregate-validation-compatibility", "5", "scripts/proof/aggregate_compat_v5.py"),
    ("arukellt-loop-semantics-v4", "loop-proof-validator", "4", "scripts/proof/loop_semantics.py"),
    ("arukellt-call-interface-binding-v3", "callee-interface-binding", "3", "scripts/proof/verified_core_interface.py"),
    ("arukellt-proof-vc-v4", "loop-vc-renderer", "4", "scripts/proof/proof_phase4_vc.py"),
    ("arukellt-smt-datatype-v1", "aggregate-smt-renderer", "1", "scripts/proof/proof_phase5_vc.py"),
    ("arukellt-typed-smt-v5", "typed-smt-adapter", "5", "scripts/proof/smtlib_typed_v5_final.py"),
    ("arukellt-proof-phase5-boundary-writer", "phase5-boundary-receipt-writer", "1", "scripts/gen/write-proof-phase5-boundary-receipt.py"),
    ("arukellt-proof-phase5-boundary-checker", "phase5-boundary-receipt-checker", "1", "scripts/check/check-proof-phase5-boundary-receipt.py"),
)

def main() -> int:
    parser = argparse.ArgumentParser(); parser.add_argument("--runtime", type=Path, required=True); parser.add_argument("--source-binding", type=Path, required=True); parser.add_argument("--output-dir", type=Path, required=True); parser.add_argument("--toolchain-output", type=Path, required=True); parser.add_argument("--z3", type=Path, required=True); args = parser.parse_args()
    subprocess.run([sys.executable, str(BASE), "--runtime", str(args.runtime), "--source-binding", str(args.source_binding), "--output-dir", str(args.output_dir), "--toolchain-output", str(args.toolchain_output), "--z3", str(args.z3)], cwd=ROOT, check=True)
    document = json.loads(args.toolchain_output.read_text(encoding="utf-8"))
    document["translator"] = {"name": "arukellt-typed-verified-core-smtlib", "version": "5", "executable": "smtlib_typed_v5_final.py"}
    document["semantic_profile"]["aggregate_model"] = "pure-algebraic-datatypes"
    document["semantic_profile"]["aggregate_encoding"] = "arukellt-smt-datatype-v1"
    for name, role, version, relative in COMPONENTS:
        source = ROOT / relative
        if not source.is_file(): raise ValueError(f"trusted component missing: {relative}")
        destination = args.output_dir / source.name
        if destination.exists() and destination.read_bytes() != source.read_bytes(): raise ValueError(f"trusted component basename collision: {relative}")
        shutil.copy2(source, destination)
        document["trusted_components"] = [entry for entry in document["trusted_components"] if entry["role"] != role]
        document["trusted_components"].append({"name": name, "role": role, "version": version, "artifact": source.name})
    args.toolchain_output.write_text(json.dumps(document, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"proof-phase5-toolchain: PASS: components={len(document['trusted_components'])} output={args.toolchain_output}")
    return 0

if __name__ == "__main__":
    try: raise SystemExit(main())
    except (OSError, ValueError, subprocess.SubprocessError, json.JSONDecodeError) as exc:
        print(f"proof-phase5-toolchain: FAIL: {exc}", file=sys.stderr); raise SystemExit(1)
