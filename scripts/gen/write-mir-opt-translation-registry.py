#!/usr/bin/env python3
"""Write a hash-bound registry for MIR optimizer validation policy."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
ENABLED_PASSES = (
    {
        "name": "async_lower",
        "implementation": "src/compiler/mir_opt/async_lower/mod.ark",
        "validator": "src/compiler/mir_opt/async_lower/translation_validation.ark",
        "policy": "one-for-one AWAIT/FUTURE_NEW to LOCAL_SET substitution; all other instructions exact",
        "failure_action": "restore original block",
    },
    {
        "name": "stdlib_resolve_normal_calls",
        "implementation": "src/compiler/mir_opt/stdlib_resolve_validated.ark",
        "validator": "src/compiler/mir_opt/stdlib_resolve_translation_validation.ark",
        "support_files": ["src/compiler/mir_opt/stdlib_inline.ark"],
        "policy": "same-length call-target rewrite to the independently reconstructed fallback specialization and matching SignatureRegistry return types",
        "failure_action": "restore original block",
    },
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
DISABLED_PASSES = (
    {
        "name": "stdlib_inline",
        "implementation": "src/compiler/mir_opt/stdlib_inline.ark",
        "status": "disabled",
        "reason": "no independent call-expansion translation validator",
        "reenable_requires": "versioned validator plus fail-closed candidate restoration",
    },
)
SHARED_VALIDATOR = "src/compiler/mir_opt/translation_validation.ark"
ORCHESTRATOR = "src/compiler/mir_opt/orchestrate.ark"


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
    orchestrator = ROOT / ORCHESTRATOR
    if not shared.is_file() or not orchestrator.is_file():
        raise ValueError("optimizer validation boundary files are missing")

    enabled: list[dict[str, object]] = []
    for entry in ENABLED_PASSES:
        implementation = ROOT / str(entry["implementation"])
        validator = ROOT / str(entry["validator"])
        if not implementation.is_file() or not validator.is_file():
            raise ValueError(f"pass files missing: {entry['name']}")
        support: list[dict[str, str]] = []
        for relative in entry.get("support_files", []):
            support_path = ROOT / str(relative)
            if not support_path.is_file():
                raise ValueError(
                    f"pass support file missing: {entry['name']}: {relative}"
                )
            support.append(
                {
                    "path": str(relative),
                    "sha256": sha256(support_path),
                }
            )
        enabled.append(
            {
                **entry,
                "status": "enforced",
                "implementation_sha256": sha256(implementation),
                "validator_sha256": sha256(validator),
                "support_files": support,
            }
        )

    disabled: list[dict[str, object]] = []
    for entry in DISABLED_PASSES:
        implementation = ROOT / str(entry["implementation"])
        if not implementation.is_file():
            raise ValueError(f"disabled pass implementation missing: {entry['name']}")
        disabled.append(
            {
                **entry,
                "implementation_sha256": sha256(implementation),
            }
        )

    document = {
        "schema": "arukellt-mir-opt-translation-validator-registry",
        "schema_version": 3,
        "orchestrator": {
            "path": ORCHESTRATOR,
            "sha256": sha256(orchestrator),
        },
        "shared_instruction_validator": {
            "path": SHARED_VALIDATOR,
            "sha256": sha256(shared),
        },
        "enabled_passes": enabled,
        "disabled_passes": disabled,
        "required_enabled_passes": [entry["name"] for entry in ENABLED_PASSES],
        "status": "enforced",
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(document, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(
        "mir-opt-translation-registry: PASS: "
        f"enabled={len(enabled)} disabled={len(disabled)} output={args.output}"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError) as exc:
        print(f"mir-opt-translation-registry: FAIL: {exc}")
        raise SystemExit(1)
