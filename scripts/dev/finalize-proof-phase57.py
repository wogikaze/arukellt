#!/usr/bin/env python3
from __future__ import annotations

import json
from pathlib import Path

PIN = "d820c7321c55df9808819a154b0f6db1a2f72832"


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if old not in text:
        raise ValueError(f"{label}: marker missing")
    return text.replace(old, new, 1)


def main() -> int:
    module = Path("scripts/proof/boundary_registry.py")
    text = module.read_text(encoding="utf-8")
    if '    "proof-semantics",' not in text:
        text = replace_once(
            text,
            '    "release-authorization",\n}',
            '    "release-authorization",\n    "proof-semantics",\n}',
            "boundary registry required set",
        )
        module.write_text(text, encoding="utf-8")

    registry_path = Path("release/boundary-registry.json")
    registry = json.loads(registry_path.read_text(encoding="utf-8"))
    if "proof-semantics" not in registry["required_boundaries"]:
        registry["required_boundaries"].append("proof-semantics")
    if not any(entry["id"] == "proof-semantics" for entry in registry["boundaries"]):
        registry["boundaries"].append({
            "id": "proof-semantics",
            "artifact": {
                "schema": "arukellt-proof-phase-boundary",
                "schema_version": 1,
                "kind": "serialized-json",
            },
            "producer": {
                "commit": PIN,
                "path": "scripts/gen/write-proof-phase7-boundary-receipt.py",
                "required_tokens": ["phase6-boundary-hash-chain", "arukellt-readonly-heap-v1"],
            },
            "validator": {
                "commit": PIN,
                "path": "scripts/check/check-proof-phase7-boundary.py",
                "required_tokens": ["EXPECTED_FILES", "semantic-admission-before-SMT"],
            },
            "consumers": [{
                "commit": PIN,
                "path": "scripts/gen/prepare-proof-release-toolchain-v7.py",
                "required_tokens": ["arukellt-proof-phase7-boundary-receipt", "proof-phases-0-7@3"],
            }],
            "evidence": {
                "commit": PIN,
                "path": "scripts/check/check-proof-phase57-z3.py",
                "required_tokens": ["phase6-machine-int", "phase7-source-v3"],
            },
            "workflow": {
                "commit": PIN,
                "path": ".github/workflows/proof-phase57.yml",
                "required_tokens": ["Proof Phase 5-7", "check-proof-phase57-z3.py"],
            },
            "failure_action": "reject Phase 5-7 proof artifact before SMT or release trust consumption",
        })
    registry_path.write_text(json.dumps(registry, indent=2) + "\n", encoding="utf-8")

    tests = Path("scripts/tests/test_boundary_registry.py")
    text = tests.read_text(encoding="utf-8")
    text = text.replace(
        "test_complete_registry_validates_all_seven_boundaries",
        "test_complete_registry_validates_all_eight_boundaries",
    )
    if '            "proof-semantics",\n' not in text:
        text = replace_once(
            text,
            '            "release-authorization",\n        })',
            '            "release-authorization",\n            "proof-semantics",\n        })',
            "boundary registry test set",
        )
    text = text.replace("self.assertGreaterEqual(len(fetched), 28)", "self.assertGreaterEqual(len(fetched), 33)")
    text = text.replace('self.assertEqual(validated["boundary_count"], 7)', 'self.assertEqual(validated["boundary_count"], 8)')
    tests.write_text(text, encoding="utf-8")

    release = Path("scripts/run/proof-required-release.sh")
    text = release.read_text(encoding="utf-8")
    receipt_marker = 'BOUNDARY_REGISTRY_RECEIPT=".build/proof/boundary-registry-validation.json"\n'
    if 'PHASE7_BOUNDARY=' not in text:
        text = replace_once(
            text,
            receipt_marker,
            receipt_marker
            + 'PHASE5_BOUNDARY=".build/proof/proof-phase5-boundary.json"\n'
            + 'PHASE6_BOUNDARY=".build/proof/proof-phase6-boundary.json"\n'
            + 'PHASE7_BOUNDARY=".build/proof/proof-phase7-boundary.json"\n',
            "release receipt variables",
        )
    evidence_marker = 'python3 scripts/gen/write-corehir-body-boundary-receipt.py\n'
    if 'write-proof-phase7-boundary-receipt.py --output "$PHASE7_BOUNDARY"' not in text:
        text = replace_once(
            text,
            evidence_marker,
            evidence_marker
            + 'python3 scripts/gen/write-proof-phase5-boundary-receipt.py --output "$PHASE5_BOUNDARY"\n'
            + 'python3 scripts/check/check-proof-phase5-boundary.py "$PHASE5_BOUNDARY"\n'
            + 'python3 scripts/gen/write-proof-phase6-boundary-receipt.py --output "$PHASE6_BOUNDARY"\n'
            + 'python3 scripts/check/check-proof-phase6-boundary.py "$PHASE6_BOUNDARY"\n'
            + 'python3 scripts/gen/write-proof-phase7-boundary-receipt.py --output "$PHASE7_BOUNDARY"\n'
            + 'python3 scripts/check/check-proof-phase7-boundary.py "$PHASE7_BOUNDARY"\n',
            "release phase receipts",
        )
    old_toolchain = '''python3 scripts/gen/prepare-proof-release-toolchain.py \\
  --runtime "$RUNTIME" \\
  --source-binding "$PROOF_ROOT/source-proof-binding.json" \\
  --output-dir "$TOOLCHAIN_ROOT" \\
  --toolchain-output "$TOOLCHAIN_ROOT/toolchain.json" \\
  --z3 "$Z3_BIN"'''
    new_toolchain = '''python3 scripts/gen/prepare-proof-release-toolchain-v7.py \\
  --runtime "$RUNTIME" \\
  --source-binding "$PROOF_ROOT/source-proof-binding.json" \\
  --phase6-boundary "$PHASE6_BOUNDARY" \\
  --phase7-boundary "$PHASE7_BOUNDARY" \\
  --output-dir "$TOOLCHAIN_ROOT" \\
  --toolchain-output "$TOOLCHAIN_ROOT/toolchain.json" \\
  --z3 "$Z3_BIN"'''
    if old_toolchain in text:
        text = text.replace(old_toolchain, new_toolchain, 1)
    elif "prepare-proof-release-toolchain-v7.py" not in text:
        raise ValueError("release toolchain marker missing")
    release.write_text(text, encoding="utf-8")

    workflow = Path(".github/workflows/proof-required-release.yml")
    text = workflow.read_text(encoding="utf-8")
    if '.build/proof/proof-phase7-boundary.json' not in text:
        marker = '            .build/proof/corehir-body-boundary.json\n'
        text = replace_once(
            text,
            marker,
            marker
            + '            .build/proof/proof-phase5-boundary.json\n'
            + '            .build/proof/proof-phase6-boundary.json\n'
            + '            .build/proof/proof-phase7-boundary.json\n',
            "release upload list",
        )
        workflow.write_text(text, encoding="utf-8")

    print("phase57-finalizer: PASS")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        print(f"phase57-finalizer: FAIL: {exc}")
        raise SystemExit(1)
