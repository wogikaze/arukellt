#!/usr/bin/env python3
"""Require native selfhost v3 emission to fail closed outside Phase 7."""
from __future__ import annotations

import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CASES = (
    ("tests/verified-core/contract_phase7_reject_arithmetic.ark", "arithmetic"),
    ("tests/verified-core/contract_phase7_reject_opaque_ref.ark", "opaque-reference"),
    ("tests/verified-core/contract_phase7_reject_vec_f32.ark", "unsupported-vec-element"),
    ("tests/verified-core/contract_phase7_reject_allocation.ark", "allocation"),
)


def run(relative: str, *extra: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [str(ROOT / "scripts" / "run" / "arukellt-selfhost.sh"), "compile", relative, *extra],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )


def main() -> int:
    for relative, label in CASES:
        ordinary = run(relative)
        if ordinary.returncode != 0:
            raise ValueError(
                f"{label}: fixture must be a valid ordinary program before proof rejection\n"
                f"stdout:\n{ordinary.stdout}\nstderr:\n{ordinary.stderr}"
            )
        proof = run(relative, "--emit", "typed-corehir-v3")
        if proof.returncode == 0:
            raise ValueError(f"{label}: unsupported source unexpectedly emitted native TypedCoreHIR v3")
        diagnostic = proof.stdout + "\n" + proof.stderr
        if "TypedCoreHIR v3" not in diagnostic:
            raise ValueError(
                f"{label}: failure occurred outside native proof source boundary\n{diagnostic}"
            )
    print(
        "selfhost-phase7-rejections: PASS: "
        "ordinary_compile=accepted native_v3=fail-closed "
        "cases=arithmetic,opaque-reference,unsupported-vec-element,allocation"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError) as exc:
        print(f"selfhost-phase7-rejections: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
