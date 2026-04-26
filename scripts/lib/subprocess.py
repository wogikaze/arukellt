"""Generic subprocess helpers."""

import subprocess
import sys


def run_cmd(
    cmd: list[str],
    *,
    cwd: str | None = None,
    env: dict | None = None,
    capture: bool = True,
    dry_run: bool = False,
) -> tuple[int, str, str]:
    """Run a command and return (returncode, stdout, stderr).

    If dry_run=True, prints the intent and returns (0, "", "") without executing.
    """
    if dry_run:
        print(f"DRY-RUN: {cmd}")
        return (0, "", "")

    result = subprocess.run(
        cmd,
        cwd=cwd,
        env=env,
        capture_output=capture,
        text=True,
    )
    stdout = result.stdout if capture else ""
    stderr = result.stderr if capture else ""
    return (result.returncode, stdout, stderr)


def run_cmd_streaming(
    cmd: list[str],
    *,
    cwd: str | None = None,
    env: dict | None = None,
    dry_run: bool = False,
) -> int:
    """Run a command, streaming stdout/stderr to terminal in real time.

    Returns the returncode.
    """
    if dry_run:
        print(f"DRY-RUN: {cmd}")
        return 0

    process = subprocess.Popen(
        cmd,
        cwd=cwd,
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )
    assert process.stdout is not None
    for line in process.stdout:
        sys.stdout.write(line)
        sys.stdout.flush()
    process.wait()
    return process.returncode
