#!/usr/bin/env python3
"""Exercise compiler-emitted structural proof reference metadata."""
from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from proof.proof_type_registry import validate_document  # noqa: E402

SOURCE = "tests/verified-core/proof_type_registry_vec.ark"


def main() -> int:
    result = subprocess.run(
        [
            str(ROOT / "scripts" / "run" / "arukellt-selfhost.sh"),
            "compile",
            SOURCE,
            "--emit",
            "proof-type-registry",
        ],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        raise ValueError(
            f"proof type registry compile exited {result.returncode}\n{result.stdout}\n{result.stderr}"
        )
    candidates: list[dict] = []
    for raw in result.stdout.splitlines():
        line = raw.strip()
        if not line.startswith("{"):
            continue
        try:
            value = json.loads(line)
        except json.JSONDecodeError:
            continue
        if value.get("schema") == "arukellt-proof-type-registry":
            candidates.append(value)
    if len(candidates) != 1:
        raise ValueError(f"expected one proof type registry, found {len(candidates)}")
    document = validate_document(candidates[0])
    by_type = {entry["type_id"]: entry for entry in document["references"]}
    if set(by_type) != {101, 102, 104}:
        raise ValueError(f"unexpected structural Vec TypeIds: {sorted(by_type)}")
    expected_elements = {101: 1, 102: 2, 104: 4}
    for type_id, element_id in expected_elements.items():
        if by_type[type_id]["element_type_id"] != element_id:
            raise ValueError(
                f"TypeId {type_id}: expected element TypeId {element_id}, got {by_type[type_id]}"
            )
    print(
        "proof-type-registry-emission: PASS: "
        "identity=TypeInfo-tag-type-args-v1 references=101->1,102->2,104->4"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, TypeError, KeyError) as exc:
        print(f"proof-type-registry-emission: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
