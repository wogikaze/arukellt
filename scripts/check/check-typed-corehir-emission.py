#!/usr/bin/env python3
"""Compile a fixture through selfhost and validate emitted TypedCoreHIR."""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from proof.common import ValidationError  # noqa: E402
from proof.typed_corehir import validate_document  # noqa: E402


def main() -> int:
    source = ROOT / "tests" / "verified-core" / "identity.ark"
    command = [
        str(ROOT / "scripts" / "run" / "arukellt-selfhost.sh"),
        "compile",
        str(source.relative_to(ROOT)),
        "--emit",
        "typed-corehir",
    ]
    result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True)
    if result.returncode != 0:
        print(f"typed-corehir-emission: FAIL: compile exited {result.returncode}", file=sys.stderr)
        print(result.stdout, file=sys.stderr)
        print(result.stderr, file=sys.stderr)
        return 1

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
        print(
            f"typed-corehir-emission: FAIL: expected one artifact, found {len(candidates)}",
            file=sys.stderr,
        )
        print(result.stdout, file=sys.stderr)
        print(result.stderr, file=sys.stderr)
        return 1

    try:
        document = validate_document(candidates[0])
    except (ValueError, ValidationError) as exc:
        print(f"typed-corehir-emission: FAIL: {exc}", file=sys.stderr)
        return 1

    names = {function["name"] for function in document["functions"]}
    if not any(name.endswith("identity") for name in names):
        print("typed-corehir-emission: FAIL: identity function missing", file=sys.stderr)
        return 1
    if not any(name.endswith("main") for name in names):
        print("typed-corehir-emission: FAIL: main function missing", file=sys.stderr)
        return 1

    print(
        "typed-corehir-emission: PASS: "
        f"types={len(document['types'])} functions={len(document['functions'])}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
