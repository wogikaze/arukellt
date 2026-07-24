from __future__ import annotations

import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]


def test_native_executor_manager_contract_dry_run() -> None:
    result = subprocess.run(
        [
            sys.executable,
            "scripts/manager.py",
            "selfhost",
            "native-executor",
            "--build",
            "--dry-run",
        ],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )

    assert result.returncode == 0
    assert "native C generation" in result.stdout
    assert "two native s3 runs" in result.stdout


def test_native_executor_allow_high_rss_flag_is_wired() -> None:
    result = subprocess.run(
        [
            sys.executable,
            "scripts/manager.py",
            "selfhost",
            "native-executor",
            "--help",
        ],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode == 0
    assert "--allow-high-rss" in result.stdout


def test_native_executor_allow_high_rss_forbidden_under_ci() -> None:
    env = dict(**{k: v for k, v in __import__("os").environ.items()})
    env["CI"] = "true"
    env["GITHUB_ACTIONS"] = "true"
    result = subprocess.run(
        [
            sys.executable,
            "scripts/manager.py",
            "selfhost",
            "native-executor",
            "--build",
            "--allow-high-rss",
            "--dry-run",
        ],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
        env=env,
    )
    assert result.returncode != 0
    combined = result.stdout + result.stderr
    assert "forbidden" in combined.lower() or "escape hatch" in combined.lower()


def test_native_executor_receipt_schema_defaults() -> None:
    sys.path.insert(0, str(ROOT / "scripts"))
    from selfhost.native_executor import RECEIPT_SCHEMA_VERSION, _empty_receipt

    receipt = _empty_receipt()
    assert receipt["receipt_schema_version"] == RECEIPT_SCHEMA_VERSION
    assert "executor_run_1" in receipt and "executor_run_2" in receipt
    assert "root_liveness" in receipt
    assert "root_clear_assignments_planned" in receipt["root_liveness"]
    assert "root_peak_slots" in receipt["root_liveness"]
    assert "root_planner_peak_bytes" in receipt["root_liveness"]
    assert "root_entry_null_inits" in receipt["root_liveness"]
    assert "correctness_gate_passed" in receipt
    assert "performance_gate_passed" in receipt
    assert "strict_gate_passed" in receipt
    assert "equality_gate_applicable" in receipt
    assert "equality_status" in receipt
    assert "gc_total_mark_time_ms" in receipt["executor_run_1"]["gc"]


def test_equality_applicability_stale_reference() -> None:
    sys.path.insert(0, str(ROOT / "scripts"))
    from selfhost.native_executor import _equality_applicability

    applicable, status = _equality_applicability(
        {"source_commit": "aaa", "native_runtime_hash": "r1"},
        current_commit="bbb",
        current_runtime_hash="r1",
        current_profile_fingerprint="fp1",
    )
    assert applicable is False
    assert status == "NOT_APPLICABLE_STALE_REFERENCE"

    applicable_ok, status_ok = _equality_applicability(
        {
            "source_commit": "aaa",
            "native_runtime_hash": "r1",
            "source_fingerprint": "fp1",
            "promoted_from_s3_sha256": "deadbeef",
        },
        current_commit="aaa",
        current_runtime_hash="r1",
        current_profile_fingerprint="fp1",
    )
    assert applicable_ok is True
    assert status_ok == "APPLICABLE"
