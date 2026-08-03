#!/usr/bin/env python3
"""Require every public solver result path to emit and validate SolverResult v1."""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DRIVER = ROOT / "scripts" / "proof" / "solver_driver.py"
RESULT = ROOT / "scripts" / "proof" / "solver_result.py"
RUNNER = ROOT / "scripts" / "run" / "run-proof-solver.py"
CAPTURED = ROOT / "scripts" / "gen" / "write-solver-receipts.py"
CHECKER = ROOT / "scripts" / "check" / "check-solver-result.py"
WORKFLOWS = ROOT / ".github" / "workflows"


def require(text: str, token: str, label: str) -> None:
    if token not in text:
        raise ValueError(f"missing {label}: {token}")


def _require_result_argument_near_calls(path: Path, command: str) -> None:
    lines = path.read_text(encoding="utf-8").splitlines()
    for index, line in enumerate(lines):
        if command not in line:
            continue
        window = "\n".join(lines[index : index + 24])
        if "--solver-result-output" not in window:
            raise ValueError(
                f"{path.relative_to(ROOT)}:{index + 1}: {command} lacks --solver-result-output"
            )


def _reject_production_bypasses() -> None:
    allowed_receipt_generators = {
        "scripts/proof/solver_driver.py",
        "scripts/gen/write-solver-receipts.py",
    }
    allowed_driver_callers = {
        "scripts/run/run-proof-solver.py",
    }
    violations: list[str] = []
    for path in sorted((ROOT / "scripts").rglob("*.py")):
        relative = path.relative_to(ROOT).as_posix()
        if relative.startswith("scripts/tests/") or relative == "scripts/check/check-solver-trust-manifest-boundary.py":
            continue
        text = path.read_text(encoding="utf-8")
        if "generate_solver_receipts" in text and relative not in allowed_receipt_generators:
            violations.append(f"{relative}: generates detached TrustManifest/ProofReceipt")
        if "run_solver_and_generate_receipts" in text and relative not in allowed_driver_callers | {"scripts/proof/solver_driver.py"}:
            violations.append(f"{relative}: bypasses canonical solver runner")
    if violations:
        raise ValueError("\n".join(violations))


def main() -> int:
    for path in (DRIVER, RESULT, RUNNER, CAPTURED, CHECKER):
        if not path.is_file():
            raise ValueError(f"solver trust boundary file missing: {path.relative_to(ROOT)}")

    driver = DRIVER.read_text(encoding="utf-8")
    result = RESULT.read_text(encoding="utf-8")
    runner = RUNNER.read_text(encoding="utf-8")
    captured = CAPTURED.read_text(encoding="utf-8")

    for token, label in (
        ("create_solver_result", "driver result construction"),
        ("write_solver_result", "driver result write"),
        ("validate_solver_result_file", "driver independent result validation"),
        ("solver_result_path", "driver mandatory result path"),
    ):
        require(driver, token, label)

    for token, label in (
        ('SCHEMA = "arukellt-solver-result"', "versioned solver result schema"),
        ('"trust_manifest"', "embedded TrustManifest"),
        ('"proof_receipt"', "embedded ProofReceipt"),
        ('"solver_input_sha256"', "solver input binding"),
        ('"toolchain_sha256"', "toolchain binding"),
        ("validate_bound_proof", "external artifact binding"),
    ):
        require(result, token, label)

    for text, label in ((runner, "solver runner"), (captured, "captured-output converter")):
        require(text, '"--solver-result-output"', f"{label} result argument")
        require(text, "required=True", f"{label} mandatory result argument")

    _reject_production_bypasses()
    for workflow in sorted((*WORKFLOWS.glob("*.yml"), *WORKFLOWS.glob("*.yaml"))):
        text = workflow.read_text(encoding="utf-8")
        if "run-proof-solver.py" in text:
            _require_result_argument_near_calls(workflow, "run-proof-solver.py")
        if "write-solver-receipts.py" in text:
            _require_result_argument_near_calls(workflow, "write-solver-receipts.py")
        if ("run-proof-solver.py" in text or "write-solver-receipts.py" in text) and "solver-result" not in text:
            raise ValueError(f"{workflow.relative_to(ROOT)}: solver workflow does not retain SolverResult")

    print(
        "solver-trust-manifest-boundary: PASS: "
        "every public solver path emits SolverResult v1 with embedded TrustManifest"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError) as exc:
        print(f"solver-trust-manifest-boundary: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
