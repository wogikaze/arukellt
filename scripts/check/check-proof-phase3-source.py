#!/usr/bin/env python3
"""Exercise Phase 3 from source through TypedCoreHIR, direct-call lowering, and SMT."""
from __future__ import annotations
import json, subprocess, sys
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path: sys.path.insert(0, str(SCRIPTS))
from proof.normalize_source_contract_profile import normalize_document
from proof.smtlib_typed_v1 import generate_typed_smtlib
from proof.typed_corehir import validate_document as validate_typed_corehir
from proof.typed_corehir_typed_convert import convert_typed_document
SOURCE = "tests/verified-core/phase3_modular.ark"
def _compile_source():
    result = subprocess.run([str(ROOT / "scripts/run/arukellt-selfhost.sh"), "compile", SOURCE, "--emit", "typed-corehir"], cwd=ROOT, capture_output=True, text=True)
    if result.returncode != 0: raise ValueError(f"source compile exited {result.returncode}\n{result.stdout}\n{result.stderr}")
    candidates = []
    for line in result.stdout.splitlines():
        stripped = line.strip()
        if not stripped.startswith("{"): continue
        try: value = json.loads(stripped)
        except json.JSONDecodeError: continue
        if value.get("schema") == "arukellt-typed-corehir": candidates.append(value)
    if len(candidates) != 1: raise ValueError(f"expected one TypedCoreHIR artifact, found {len(candidates)}")
    return validate_typed_corehir(candidates[0])
def main():
    typed_corehir = _compile_source()
    verified = convert_typed_document(typed_corehir)
    names = {str(function["name"]).split("::")[-1] for function in verified["functions"]}
    expected = {"nonnegative_identity", "forward_nonnegative"}
    if not expected <= names: raise ValueError(f"contracted functions missing: {sorted(expected - names)}")
    calls = [instruction for function in verified["functions"] for block in function["body"]["blocks"] for instruction in block["instructions"] if instruction["op"] == "call"]
    if not calls: raise ValueError("direct source call was not lowered")
    rendered = generate_typed_smtlib(normalize_document(verified))
    if "callee-requires" not in rendered: raise ValueError("callee requires obligation missing")
    print(f"proof-phase3-source: PASS: functions={len(verified['functions'])} calls={len(calls)} obligations={rendered.count('(check-sat)')}")
    return 0
if __name__ == "__main__":
    try: raise SystemExit(main())
    except (OSError, ValueError, TypeError, KeyError, subprocess.SubprocessError) as exc:
        print(f"proof-phase3-source: FAIL: {exc}", file=sys.stderr); raise SystemExit(1)
