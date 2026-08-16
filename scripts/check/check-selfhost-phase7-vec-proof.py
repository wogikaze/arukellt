#!/usr/bin/env python3
"""Prove a selfhost-emitted read-only Vec contract through canonical Phase 7."""
from __future__ import annotations

import json
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from proof.smtlib_typed_v7 import generate_typed_smtlib  # noqa: E402
from proof.typed_corehir_v1_scalar_v3 import upgrade_scalar_document  # noqa: E402
from proof.typed_corehir_v3_convert import convert_document  # noqa: E402

SOURCE = "tests/verified-core/contract_vec_read.ark"
OUTPUT = ROOT / ".build" / "typed-contract-proof" / "vec-phase7"


def compile_source() -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [
            str(ROOT / "scripts" / "run" / "arukellt-selfhost.sh"),
            "compile",
            SOURCE,
            "--emit",
            "typed-corehir",
        ],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )


def emitted_artifact(result: subprocess.CompletedProcess[str]) -> dict:
    values: list[dict] = []
    for raw in result.stdout.splitlines():
        line = raw.strip()
        if not line.startswith("{"):
            continue
        try:
            value = json.loads(line)
        except json.JSONDecodeError:
            continue
        if value.get("schema") == "arukellt-typed-corehir":
            values.append(value)
    if len(values) != 1:
        raise ValueError(f"expected one TypedCoreHIR artifact, found {len(values)}")
    return values[0]


def main() -> int:
    result = compile_source()
    if result.returncode != 0:
        raise ValueError(
            f"selfhost Vec proof compile exited {result.returncode}\n{result.stdout}\n{result.stderr}"
        )
    raw = emitted_artifact(result)
    function = next(
        (entry for entry in raw.get("functions", []) if str(entry.get("name", "")).endswith("array_at")),
        None,
    )
    if not isinstance(function, dict):
        raise ValueError("array_at missing from selfhost TypedCoreHIR")
    parameter_types = [int(entry["type_id"]) for entry in function["signature"]["parameters"]]
    if parameter_types != [101, 1]:
        raise ValueError(f"unexpected selfhost parameter TypeIds: {parameter_types}")
    raw_reference = next((entry for entry in raw["types"] if entry.get("id") == 101), None)
    if not isinstance(raw_reference, dict):
        raise ValueError("compiler structural Vec TypeId 101 missing from raw type table")
    if raw_reference.get("name") != "proof-vec-1":
        raise ValueError(f"unexpected non-semantic Vec display name: {raw_reference.get('name')!r}")

    canonical = upgrade_scalar_document(raw)
    if canonical["proof_memory"]["references"] != [{
        "type_id": 101,
        "kind": "array",
        "element_type_id": 1,
        "length_type_id": 1,
    }]:
        raise ValueError(f"unexpected canonical reference descriptor: {canonical['proof_memory']}")
    kinds = {
        expression["kind"]
        for expression in function["body"]["expressions"]
    }
    if "index" not in kinds or "method-call" not in kinds:
        raise ValueError(f"raw Vec proof did not retain index/len expressions: {sorted(kinds)}")

    verified = convert_document(canonical)
    rendered = generate_typed_smtlib(verified)
    if "arukellt_heap_t101_get" not in rendered or "arukellt_heap_t101_len" not in rendered:
        raise ValueError("Phase 7 SMT does not encode structural Vec heap reads")

    z3 = shutil.which("z3")
    if z3 is None:
        raise ValueError("z3 not found")
    solved = subprocess.run(
        [z3, "-in", "-smt2"],
        input=rendered,
        capture_output=True,
        text=True,
        cwd=ROOT,
    )
    if solved.returncode != 0:
        raise ValueError(f"z3 exited {solved.returncode}: {solved.stderr}")
    statuses = [line.strip() for line in solved.stdout.splitlines() if line.strip()]
    if not statuses or any(status != "unsat" for status in statuses):
        raise ValueError(f"Vec Phase 7 proof did not fully prove: {statuses}")

    OUTPUT.mkdir(parents=True, exist_ok=True)
    (OUTPUT / "typed-corehir-raw.json").write_text(
        json.dumps(raw, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    (OUTPUT / "typed-corehir-v3.json").write_text(
        json.dumps(canonical, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    (OUTPUT / "verified-core.json").write_text(
        json.dumps(verified, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    (OUTPUT / "verified-core-vcs.smt2").write_text(rendered, encoding="utf-8")
    (OUTPUT / "z3-output.txt").write_text(solved.stdout, encoding="utf-8")
    print(
        "selfhost-phase7-vec-proof: PASS: "
        f"type_id=101 element_type_id=1 obligations={len(statuses)} output={OUTPUT}"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, TypeError, KeyError, json.JSONDecodeError) as exc:
        print(f"selfhost-phase7-vec-proof: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
