#!/usr/bin/env python3
"""Phase 3 facade for release proof toolchain construction."""

from __future__ import annotations

import json
import sys
from pathlib import Path

import prepare_proof_release_toolchain_impl as _impl

_REMOVE = {
    "arukellt-typed-corehir-converter-v2",
    "arukellt-typed-corehir-converter-impl-v1",
    "arukellt-source-contract-profile-normalizer-v1",
    "arukellt-verified-core-semantic-validator-v1",
    "arukellt-smt-renderer-v1",
}
_EXTRA = (
    ("arukellt-proof-capability-profile-v1", "proof-capability-profile", "1", "docs/data/proof-capabilities-v1.json"),
    ("arukellt-proof-capability-validator-v1", "proof-capability-validator", "1", "scripts/proof/capabilities.py"),
    ("arukellt-typed-corehir-converter-v3", "typed-proof-boundary", "3", "scripts/proof/typed_corehir_typed_convert.py"),
    ("arukellt-typed-corehir-program-lowering-v3", "typed-proof-program-lowering", "3", "scripts/proof/typed_corehir_program_convert.py"),
    ("arukellt-source-contract-profile-normalizer-v2", "semantic-profile-normalizer", "2", "scripts/proof/normalize_source_contract_profile.py"),
    ("arukellt-verified-core-program-validator-v1", "artifact-validator-implementation", "1", "scripts/proof/verified_core_program.py"),
    ("arukellt-verified-core-semantic-validator-v2", "typed-artifact-validator", "2", "scripts/proof/verified_core_typed.py"),
    ("arukellt-verified-core-semantic-validator-impl-v2", "typed-artifact-validator-implementation", "2", "scripts/proof/verified_core_typed_impl.py"),
    ("arukellt-smt-renderer-v3", "smt-renderer-implementation", "3", "scripts/proof/smtlib_v1.py"),
    ("arukellt-typed-boundary-receipt-validator-impl-v1", "typed-boundary-receipt-validator-implementation", "1", "scripts/proof/typed_verified_core_receipt_impl.py"),
)

_impl.COMPONENTS = tuple(entry for entry in _impl.COMPONENTS if entry[0] not in _REMOVE) + _EXTRA


def _toolchain_output() -> Path:
    if "--toolchain-output" not in sys.argv:
        raise ValueError("--toolchain-output is required")
    index = sys.argv.index("--toolchain-output")
    if index + 1 >= len(sys.argv):
        raise ValueError("--toolchain-output requires a path")
    return Path(sys.argv[index + 1])


def main() -> int:
    output = _toolchain_output()
    result = _impl.main()
    if result != 0:
        return result
    document = json.loads(output.read_text(encoding="utf-8"))
    document["translator"]["version"] = "3"
    output.write_text(json.dumps(document, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"proof-release-toolchain-phase3: PASS: components={len(document['trusted_components'])}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        print(f"proof-release-toolchain: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
