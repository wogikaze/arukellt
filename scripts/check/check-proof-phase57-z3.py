#!/usr/bin/env python3
"""Run focused real-Z3 proof smokes for proof Phases 6 and 7."""
from __future__ import annotations

import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from proof.smtlib_typed_v6 import generate_typed_smtlib as generate_v6
from proof.smtlib_typed_v7 import generate_typed_smtlib as generate_v7
from proof.typed_corehir_v3_convert import convert_document
from scripts.tests.test_proof_phase6_machine_int import _base_document
from scripts.tests.test_proof_phase7_readonly_memory import _array_function, _base, _object_function
from scripts.tests.test_typed_corehir_v3_machine_memory import source_document


def _run(name: str, smt: str) -> int:
    obligations = smt.count("(check-sat)")
    if obligations < 1:
        raise ValueError(f"{name}: no obligations")
    completed = subprocess.run(
        ["z3", "-in", "-smt2"],
        input=smt,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=30,
        check=False,
    )
    if completed.returncode != 0:
        raise ValueError(f"{name}: z3 exit={completed.returncode}: {completed.stderr.strip()}")
    if completed.stderr.strip():
        raise ValueError(f"{name}: z3 stderr: {completed.stderr.strip()}")
    statuses = [line.strip() for line in completed.stdout.splitlines() if line.strip() in {"sat", "unsat", "unknown"}]
    unexpected = [line for line in completed.stdout.splitlines() if line.strip() and line.strip() not in {"sat", "unsat", "unknown"}]
    if unexpected:
        raise ValueError(f"{name}: unexpected z3 output: {unexpected[:3]}")
    if len(statuses) != obligations:
        raise ValueError(f"{name}: status count {len(statuses)} != obligations {obligations}")
    if any(status != "unsat" for status in statuses):
        raise ValueError(f"{name}: proof failed: {statuses}")
    print(f"{name}: PASS: obligations={obligations}")
    return obligations


def main() -> int:
    total = 0
    total += _run("phase6-machine-int", generate_v6(_base_document()))

    object_document = _base()
    object_document["functions"] = [_object_function()]
    total += _run("phase7-object-read", generate_v7(object_document))

    array_document = _base()
    array_document["functions"] = [_array_function()]
    total += _run("phase7-array-read", generate_v7(array_document))

    total += _run("phase7-source-v3", generate_v7(convert_document(source_document())))
    print(f"proof-phase57-z3: PASS: obligations={total}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, subprocess.SubprocessError) as exc:
        print(f"proof-phase57-z3: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
