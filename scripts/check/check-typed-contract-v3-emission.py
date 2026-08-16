#!/usr/bin/env python3
"""Exercise native selfhost TypedCoreHIR v3 emission for source contracts."""
from __future__ import annotations

import json
import sys
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
OUTPUT = ROOT / ".build" / "typed-contract-proof" / "typed-corehir-v3.json"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from proof.smtlib_typed_v7 import generate_typed_smtlib  # noqa: E402
from proof.typed_corehir_v3 import validate_document  # noqa: E402
from proof.typed_corehir_v3_convert import convert_document  # noqa: E402

SOURCE = "tests/verified-core/contract_identity.ark"
GENERATOR = "arukellt-selfhost-typed-corehir-v3"


def compile_source() -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [
            str(ROOT / "scripts" / "run" / "arukellt-selfhost.sh"),
            "compile",
            SOURCE,
            "--emit",
            "typed-corehir-v3",
        ],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )


def emitted_artifact(result: subprocess.CompletedProcess[str]) -> dict:
    candidates: list[dict] = []
    for raw in result.stdout.splitlines():
        line = raw.strip()
        if not line.startswith("{"):
            continue
        try:
            value = json.loads(line)
        except json.JSONDecodeError:
            continue
        if value.get("schema") == "arukellt-typed-corehir" and value.get("schema_version") == 3:
            candidates.append(value)
    if len(candidates) != 1:
        raise ValueError(f"expected one native TypedCoreHIR v3 artifact, found {len(candidates)}")
    return validate_document(candidates[0])


def main() -> int:
    result = compile_source()
    if result.returncode != 0:
        raise ValueError(
            f"native v3 contract compile exited {result.returncode}\n{result.stdout}\n{result.stderr}"
        )
    document = emitted_artifact(result)
    if document.get("generator") != GENERATOR:
        raise ValueError(f"unexpected native v3 generator: {document.get('generator')!r}")
    if document.get("target_profile") != {
        "integer_model": "machine",
        "overflow": "checked",
        "floating_point": "unsupported",
        "pointer_width": document["target_profile"]["pointer_width"],
    }:
        raise ValueError(f"unexpected native v3 target profile: {document['target_profile']}")
    if document.get("proof_memory") != {
        "model": "arukellt-readonly-heap-v1",
        "references": [],
    }:
        raise ValueError(f"scalar native v3 unexpectedly carries references: {document['proof_memory']}")
    identity = next(
        (function for function in document["functions"] if str(function["name"]).endswith("identity")),
        None,
    )
    if not isinstance(identity, dict):
        raise ValueError("identity missing from native TypedCoreHIR v3")
    if [contract["kind"] for contract in identity["contracts"]] != ["requires", "ensures"]:
        raise ValueError(f"unexpected native v3 contracts: {identity['contracts']}")
    kinds = {expression["kind"] for expression in identity["body"]["expressions"]}
    if not {"block", "local", "constant", "ge"} <= kinds:
        raise ValueError(f"native v3 scalar expression surface incomplete: {sorted(kinds)}")

    verified = convert_document(document)
    rendered = generate_typed_smtlib(verified)
    if "(check-sat)" not in rendered or "machine" not in rendered:
        raise ValueError("native v3 source did not reach Phase 7 SMT")

    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(json.dumps(document, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(
        "typed-contract-v3-emission: PASS: "
        f"generator={GENERATOR} functions={len(document['functions'])} output={OUTPUT}"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError) as exc:
        print(f"typed-contract-v3-emission: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
