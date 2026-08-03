"""Run a configured proof solver and emit one complete solver result artifact."""

from __future__ import annotations

import os
import subprocess
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from proof.solver_receipts import generate_solver_receipts, load_toolchain
from proof.solver_result import (
    create_solver_result,
    validate_solver_result_file,
    write_solver_result,
)


@dataclass(frozen=True)
class SolverRunResult:
    process_returncode: int
    proof_status: str
    obligation_count: int
    solver_output_path: Path
    trust_manifest_path: Path
    proof_receipt_path: Path
    solver_result_path: Path


def _string(value: Any, label: str) -> str:
    if not isinstance(value, str) or not value:
        raise ValueError(f"{label}: expected non-empty string")
    return value


def _positive_int(value: Any, label: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or value < 1:
        raise ValueError(f"{label}: expected positive integer")
    return value


def _solver_executable(toolchain_path: Path, toolchain: dict[str, Any]) -> Path:
    solver = toolchain.get("solver")
    if not isinstance(solver, dict):
        raise ValueError("toolchain.solver: expected object")
    raw = Path(_string(solver.get("executable"), "toolchain.solver.executable"))
    if raw.is_absolute() or ".." in raw.parts:
        raise ValueError("toolchain.solver.executable: expected relative path without '..'")
    base = toolchain_path.parent.resolve()
    resolved = (base / raw).resolve()
    if resolved != base and base not in resolved.parents:
        raise ValueError("toolchain.solver.executable: resolved path escapes toolchain directory")
    if not resolved.is_file():
        raise ValueError(f"toolchain.solver.executable: missing file: {resolved}")
    if not os.access(resolved, os.X_OK):
        raise ValueError(f"toolchain.solver.executable: not executable: {resolved}")
    return resolved


def _solver_arguments(toolchain: dict[str, Any]) -> list[str]:
    solver = toolchain["solver"]
    raw = solver.get("arguments", [])
    if not isinstance(raw, list) or any(not isinstance(item, str) for item in raw):
        raise ValueError("toolchain.solver.arguments: expected string array")
    return list(raw)


def _limit_memory(memory_bytes: int):
    if os.name != "posix":
        return None

    def apply() -> None:
        import resource

        resource.setrlimit(resource.RLIMIT_AS, (memory_bytes, memory_bytes))

    return apply


def _captured_output(stdout: str, stderr: str, returncode: int) -> str:
    lines: list[str] = []
    lines.extend(line for line in stdout.splitlines() if line.strip())
    for line in stderr.splitlines():
        stripped = line.strip()
        if stripped:
            lines.append(f"(error stderr: {stripped})")
    if returncode != 0 and not any(line.strip().lower().startswith("(error") for line in lines):
        lines.append(f"(error solver exit {returncode})")
    return "\n".join(lines) + ("\n" if lines else "")


def run_solver_and_generate_receipts(
    subject_path: Path,
    solver_input_path: Path,
    toolchain_path: Path,
    solver_output_path: Path,
    trust_manifest_path: Path,
    proof_receipt_path: Path,
    solver_result_path: Path,
) -> SolverRunResult:
    toolchain = load_toolchain(toolchain_path)
    executable = _solver_executable(toolchain_path, toolchain)
    arguments = _solver_arguments(toolchain)
    limits = toolchain.get("limits")
    if not isinstance(limits, dict):
        raise ValueError("toolchain.limits: expected object")
    timeout_ms = _positive_int(limits.get("timeout_ms"), "toolchain.limits.timeout_ms")
    memory_bytes = _positive_int(limits.get("memory_bytes"), "toolchain.limits.memory_bytes")
    solver_input = solver_input_path.read_text(encoding="utf-8")

    timed_out = False
    try:
        completed = subprocess.run(
            [str(executable), *arguments],
            input=solver_input,
            cwd=toolchain_path.parent,
            capture_output=True,
            text=True,
            timeout=timeout_ms / 1000,
            check=False,
            preexec_fn=_limit_memory(memory_bytes),
        )
        returncode = completed.returncode
        captured = _captured_output(completed.stdout, completed.stderr, returncode)
    except subprocess.TimeoutExpired as exc:
        timed_out = True
        returncode = 124
        stdout = exc.stdout.decode() if isinstance(exc.stdout, bytes) else (exc.stdout or "")
        stderr = exc.stderr.decode() if isinstance(exc.stderr, bytes) else (exc.stderr or "")
        captured = _captured_output(stdout, stderr, returncode)
        captured += "(error solver timeout)\n"

    solver_output_path.parent.mkdir(parents=True, exist_ok=True)
    solver_output_path.write_text(captured, encoding="utf-8")
    _, receipt = generate_solver_receipts(
        subject_path,
        solver_output_path,
        toolchain_path,
        trust_manifest_path,
        proof_receipt_path,
    )
    status = str(receipt["status"])
    if timed_out and status != "error":
        raise ValueError("timed-out solver run did not produce status=error")

    solver_result = create_solver_result(
        subject_path=subject_path,
        solver_input_path=solver_input_path,
        toolchain_path=toolchain_path,
        solver_output_path=solver_output_path,
        trust_manifest_path=trust_manifest_path,
        proof_receipt_path=proof_receipt_path,
        execution_mode="solver-process",
        process_returncode=returncode,
        timed_out=timed_out,
    )
    write_solver_result(solver_result, solver_result_path)
    validate_solver_result_file(
        solver_result_path,
        subject_path=subject_path,
        solver_input_path=solver_input_path,
        toolchain_path=toolchain_path,
        solver_output_path=solver_output_path,
        trust_manifest_path=trust_manifest_path,
        proof_receipt_path=proof_receipt_path,
    )

    return SolverRunResult(
        process_returncode=returncode,
        proof_status=status,
        obligation_count=int(receipt["obligations"]["total"]),
        solver_output_path=solver_output_path,
        trust_manifest_path=trust_manifest_path,
        proof_receipt_path=proof_receipt_path,
        solver_result_path=solver_result_path,
    )


__all__ = ["SolverRunResult", "run_solver_and_generate_receipts"]
