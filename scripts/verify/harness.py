"""Verify-specific harness integration."""

from pathlib import Path

GREEN = "\033[0;32m"
RED = "\033[0;31m"
YELLOW = "\033[1;33m"
NC = "\033[0m"


class Harness:
    def __init__(
        self,
        repo_root: Path,
        dry_run: bool = False,
        arukellt_bin: str = "",
        arukellt_target: str = "",
    ) -> None:
        self.repo_root = repo_root
        self.dry_run = dry_run
        self.arukellt_bin = arukellt_bin
        self.arukellt_target = arukellt_target
        self._total = 0
        self._passed = 0
        self._skipped = 0

    # ── counters ────────────────────────────────────────────────────────────

    @property
    def total(self) -> int:
        return self._total

    @property
    def passed(self) -> int:
        return self._passed

    @property
    def skipped(self) -> int:
        return self._skipped

    @property
    def failed(self) -> int:
        return self._total - self._passed - self._skipped

    # ── check helpers ────────────────────────────────────────────────────────

    def check_pass(self, label: str) -> None:
        print(f"{GREEN}\u2713 {label}{NC}")
        self._passed += 1
        self._total += 1

    def check_fail(self, label: str) -> None:
        print(f"{RED}\u2717 {label}{NC}")
        self._total += 1

    def check_skip(self, label: str) -> None:
        print(f"{YELLOW}\u2299 {label} (skipped){NC}")
        self._skipped += 1
        self._total += 1

    # ── run_check ────────────────────────────────────────────────────────────

    def run_check(self, label: str, cmd: list[str], tail_lines: int = 30) -> bool:
        """Run cmd; print pass/fail with last tail_lines of output on failure.

        Returns True on pass, False on fail.
        """
        import subprocess as _sp

        if self.dry_run:
            print(f"DRY-RUN: {cmd}")
            self.check_pass(label)
            return True

        result = _sp.run(
            cmd,
            cwd=str(self.repo_root),
            capture_output=True,
            text=True,
        )
        if result.returncode == 0:
            self.check_pass(label)
            return True
        else:
            self.check_fail(label)
            combined = (result.stdout + result.stderr).splitlines()
            tail = combined[-tail_lines:]
            for line in tail:
                print(line)
            return False

    # ── summary / exit ────────────────────────────────────────────────────────

    def summary(self) -> tuple[int, int, int, int]:
        """Return (total, passed, skipped, failed)."""
        return (self._total, self._passed, self._skipped, self.failed)

    def exit_code(self) -> int:
        """Return 0 if all checks passed or skipped, else 1."""
        return 0 if (self._passed + self._skipped) == self._total else 1
