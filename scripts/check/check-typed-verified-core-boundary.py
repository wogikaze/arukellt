#!/usr/bin/env python3
"""Check the versioned typed proof boundary from legacy phases through Phase 7."""

from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SELF = "scripts/check/check-typed-verified-core-boundary.py"


def require(path: str, token: str) -> None:
    text = (ROOT / path).read_text(encoding="utf-8")
    if token not in text:
        raise ValueError(f"{path}: required token missing: {token}")


def main() -> int:
    required = {
        "scripts/gen/convert-typed-corehir.py": ["proof.typed_corehir_typed_convert", "convert_typed_document"],
        "scripts/proof/typed_corehir_typed_convert.py": ["Explicit logical type conversion facade", "typed_corehir_phase4_convert", "bind_call_interfaces", "validate_typed_document(result)"],
        "scripts/proof/typed_corehir_phase4_convert.py": ["Phase4Lowerer", "proof while requires invariant and decreases annotations", "validate_typed_document(result)"],
        "scripts/proof/typed_corehir_program_convert.py": ["arukellt-typed-corehir-converter-v3", "expected 32 or 64", "only signed integers are supported", 'kind == "if"', 'kind == "call"', "indirect calls are outside phase 3", "validate_typed_document(result)"],
        "scripts/proof/verified_core_program.py": ["INSTRUCTION_OPS", "TERMINATORS", "unsupported instruction"],
        "scripts/proof/verified_core_typed.py": ["typed_admission_v4", "validate_typed_document"],
        "scripts/proof/typed_admission_v4.py": ["verified_core_typed_impl", "verified_core_typed_phase4", "_precheck_legacy_diagnostics", "_validate_call_interface_digests", "validate_loop_semantics"],
        "scripts/proof/verified_core_typed_impl.py": ["verified_core_program", "cyclic CFG is outside proof phase 2", "call signature mismatch", "recursive proof calls are outside phase 3", "contract must be bool"],
        "scripts/proof/smtlib_typed_v1.py": ["validate_typed_document(value)", "generate_smtlib(document)"],
        "scripts/proof/smtlib_v1.py": ["callee-requires", "local_facts.append", "path_conditions", 'block["parameters"]'],
        "scripts/proof/typed_verified_core_receipt.py": ["straight-line-instruction-typing", "acyclic-cfg-edge-typing", "direct-call-contract-typing", "recursive-call-rejection"],
        "scripts/check/check-proof-phase3-modular.py": ["callee requires did not become a caller obligation", "modular call value"],
        ".github/workflows/typed-corehir-proof-pipeline.yml": ["scripts.tests.test_proof_program_phases", "scripts.tests.test_proof_phase3_calls", "typed_corehir_program_convert.py", "verified_core_program.py"],
        "scripts/proof/typed_corehir_v1_scalar_v3.py": ["arukellt-selfhost-v1-scalar-upgrade-v1", "overflow-capable or unsupported", "proof_memory"],
        "scripts/proof/typed_corehir_v3_convert.py": ["arukellt-typed-corehir-converter-v7", "validate_typed_document"],
        "scripts/gen/convert-typed-corehir-v7.py": ["typed_corehir_v3_convert", "convert_document"],
        "scripts/proof/smtlib_typed_v7.py": ["typed_admission_v7", "generate_smtlib"],
        "scripts/gen/write-smt-vcs-v7.py": ["smtlib_typed_v7", "generate_typed_smtlib_file"],
    }
    for path, tokens in required.items():
        for token in tokens:
            require(path, token)

    manifest = json.loads((ROOT / "docs/data/proof-capabilities-v1.json").read_text(encoding="utf-8"))
    if set(manifest.get("phases", {})) != {"0", "1", "2", "3"}:
        raise ValueError("proof capability manifest v1 must define exactly phases 0-3")
    phase7 = json.loads((ROOT / "docs/data/proof-capabilities-v3.json").read_text(encoding="utf-8"))
    if set(phase7.get("phases", {})) != {"0", "1", "2", "3", "4", "5", "6", "7"}:
        raise ValueError("proof capability manifest v3 must define exactly phases 0-7")

    violations = []
    for path in sorted((ROOT / "scripts").rglob("*.py")):
        relative = path.relative_to(ROOT).as_posix()
        if relative.startswith("scripts/tests/") or relative == SELF:
            continue
        text = path.read_text(encoding="utf-8")
        if "from proof.typed_corehir_convert import" in text:
            violations.append(f"{relative}: legacy converter import is not a production path")
        if "from proof.smtlib_v1 import" in text and relative != "scripts/proof/smtlib_typed_v1.py":
            violations.append(f"{relative}: SMT rendering must enter through typed admission")
    if violations:
        raise ValueError("\n".join(violations))

    print("typed-verified-core-boundary: PASS: phases=0..7 versioned facades enforced")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        print(f"typed-verified-core-boundary: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
