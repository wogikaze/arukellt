#!/usr/bin/env python3
"""Exercise function proof contracts from source syntax through Proof IR."""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from scripts.proof.ir import ValidationError, validate_document  # noqa: E402

WRAPPER = ROOT / "scripts" / "run" / "arukellt-selfhost.sh"
CASES = (
    ("valid.ark", True, None),
    (
        "requires-not-bool.ark.txt",
        False,
        "proof requires contract must have type bool, got i32",
    ),
    (
        "ensures-not-bool.ark.txt",
        False,
        "proof ensures contract must have type bool, got i32",
    ),
)


def run_command(*args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["bash", str(WRAPPER), *args],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=False,
    )


def run_case(name: str, should_pass: bool, expected: str | None) -> list[str]:
    source = ROOT / "tests" / "proof-contracts" / name
    completed = run_command("check", str(source))
    output = completed.stdout + completed.stderr
    errors: list[str] = []
    if should_pass and completed.returncode != 0:
        errors.append(f"{name}: expected success, got {completed.returncode}\n{output}")
    if not should_pass and completed.returncode == 0:
        errors.append(f"{name}: expected failure, command succeeded")
    if expected is not None and expected not in output:
        errors.append(f"{name}: missing diagnostic {expected!r}\n{output}")
    return errors


def extract_proof_ir(output: str) -> dict[str, object]:
    for line in output.splitlines():
        if line.startswith('{"schema":"arukellt-proof-ir"'):
            document = json.loads(line)
            return validate_document(document)
    raise ValidationError("$", "Proof IR JSON line was not emitted")


def run_artifact_case() -> list[str]:
    source = ROOT / "tests" / "proof-contracts" / "valid.ark"
    completed = run_command(
        "compile",
        str(source),
        "--dump-phases",
        "proof-ir",
    )
    output = completed.stdout + completed.stderr
    if completed.returncode != 0:
        return [f"proof-ir artifact: compile failed with {completed.returncode}\n{output}"]
    try:
        document = extract_proof_ir(output)
    except (json.JSONDecodeError, ValidationError) as error:
        return [f"proof-ir artifact: {error}\n{output}"]

    functions = document["functions"]
    if not isinstance(functions, list) or len(functions) != 1:
        return [f"proof-ir artifact: expected one contracted function, got {functions!r}"]
    function = functions[0]
    assert isinstance(function, dict)
    contracts = function["contracts"]
    assert isinstance(contracts, list)
    expressions = [contract["expression"] for contract in contracts]
    expected = [
        '(ge (ident "x") (int 0))',
        '(ge (ident "result") (ident "x"))',
    ]
    if expressions != expected:
        return [f"proof-ir artifact: expressions differ: {expressions!r}"]
    return []


def main() -> int:
    errors: list[str] = []
    for name, should_pass, expected in CASES:
        errors.extend(run_case(name, should_pass, expected))
    errors.extend(run_artifact_case())
    if errors:
        print("\n\n".join(errors), file=sys.stderr)
        return 1
    print(f"proof-contract frontend: {len(CASES)} source cases + Proof IR passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
