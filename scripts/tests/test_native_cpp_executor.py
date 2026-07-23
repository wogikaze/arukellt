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
