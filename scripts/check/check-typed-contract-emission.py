#!/usr/bin/env python3
"""Exercise source proof contracts through selfhost TypedCoreHIR emission."""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
OUTPUT = ROOT / ".build" / "typed-contract-proof" / "typed-corehir.json"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from proof.typed_corehir import validate_document  # noqa: E402


def compile_source(relative: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [
            str(ROOT / "scripts" / "run" / "arukellt-selfhost.sh"),
            "compile",
            relative,
            "--emit",
            "typed-corehir",
        ],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )


def emitted_artifact(result: subprocess.CompletedProcess[str]) -> dict[str, object]:
    candidates: list[dict[str, object]] = []
    for line in result.stdout.splitlines():
        stripped = line.strip()
        if not stripped.startswith("{"):
            continue
        try:
            value = json.loads(stripped)
        except json.JSONDecodeError:
            continue
        if value.get("schema") == "arukellt-typed-corehir":
            candidates.append(value)
    if len(candidates) != 1:
        raise ValueError(f"expected one TypedCoreHIR artifact, found {len(candidates)}")
    return validate_document(candidates[0])


def main() -> int:
    valid = compile_source("tests/verified-core/contract_identity.ark")
    if valid.returncode != 0:
        raise ValueError(
            f"valid contract compile exited {valid.returncode}\n{valid.stdout}\n{valid.stderr}"
        )
    document = emitted_artifact(valid)
    functions = document["functions"]
    identity = next(
        (
            function
            for function in functions
            if str(function["name"]).endswith("identity")
        ),
        None,
    )
    if identity is None:
        raise ValueError("identity function missing from TypedCoreHIR")
    contracts = identity["contracts"]
    if [contract["kind"] for contract in contracts] != ["requires", "ensures"]:
        raise ValueError(f"unexpected contract kinds: {contracts}")
    if contracts[1].get("result_name") != "result":
        raise ValueError("ensures contract does not bind result")
    expression_ids = {
        expression["id"]
        for expression in identity["body"]["expressions"]
    }
    for contract in contracts:
        if contract["expression_id"] not in expression_ids:
            raise ValueError(f"contract expression is not retained: {contract}")

    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(
        json.dumps(document, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )

    invalid = compile_source("tests/verified-core/contract_non_bool.ark")
    if invalid.returncode == 0:
        raise ValueError("non-bool contract unexpectedly compiled")
    diagnostic = invalid.stdout + "\n" + invalid.stderr
    if "proof contract must have type bool" not in diagnostic:
        raise ValueError(f"non-bool contract diagnostic missing\n{diagnostic}")

    print(
        "typed-contract-emission: PASS: "
        f"contracts={len(contracts)} expressions={len(expression_ids)} output={OUTPUT}"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, StopIteration, KeyError, TypeError) as exc:
        print(f"typed-contract-emission: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
