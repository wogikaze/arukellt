#!/usr/bin/env python3
"""Write a hash-bound registry for fail-closed MIR optimization validators."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PASSES = (
    {
        "name": "gc_hint",
        "implementation": "src/compiler/mir_opt/gc_hint_core.ark",
        "validator": "src/compiler/mir_opt/gc_hint_core.ark",
        "policy": "insert-only canonical gc.hint; all original instructions preserved",
        "failure_action": "restore original block",
    },
    {
        "name": "licm",
        "implementation": "src/compiler/mir_opt/licm_core.ark",
        "validator": "src/compiler/mir_opt/licm_translation_validation.ark",
        "policy": "ordered pure invariant hoists from one loop region; remainder exact",
        "failure_action": "discard candidate region transform",
    },
    {
        "name": "loop_unroll",
        "implementation": "src/compiler/mir_opt/loop_unroll.ark",
        "validator": "src/compiler/mir_opt/loop_unroll_translation_validation.ark",
        "policy": "exact trip expansion with counter local-get substitution only",
        "failure_action": "restore original block",
    },
)
SHARED_VALIDATOR = "src/compiler/mir_opt/translation_validation.ark"


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    digest.update(path.read_bytes())
    return digest.hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--output",
        type=Path,
        default=ROOT / ".build" / "proof" / "mir-opt-translation-registry.json",
    )
    args = parser.parse_args()

    shared = ROOT / SHARED_VALIDATOR
    if not shared.is_file():
        raise ValueError(f"shared validator missing: {SHARED_VALIDATOR}")
    entries: list[dict[str, object]] = []
    for entry in PASSES:
        implementation = ROOT / str(entry["implementation"])
        validator = ROOT / str(entry["validator"])
        if not implementation.is_file() or not validator.is_file():
            raise ValueError(f"pass files missing: {entry['name']}")
        entries.append(
            {
                **entry,
                "implementation_sha256": sha256(implementation),
                "validator_sha256": sha256(validator),
            }
        )

    document = {
        "schema": "arukellt-mir-opt-translation-validator-registry",
        "schema_version": 1,
        "shared_instruction_validator": {
            "path": SHARED_VALIDATOR,
            "sha256": sha256(shared),
        },
        "passes": entries,
        "required_passes": [entry["name"] for entry in PASSES],
        "status": "enforced",
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(document, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(
        "mir-opt-translation-registry: PASS: "
        f"passes={len(entries)} output={args.output}"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError) as exc:
        print(f"mir-opt-translation-registry: FAIL: {exc}")
        raise SystemExit(1)
