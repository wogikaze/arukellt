"""Serialize pinned-bootstrap wasmtime/selfhost compile usage across verify gates."""

from __future__ import annotations

import fcntl
import os
import sys
import time
from collections.abc import Callable
from pathlib import Path
from typing import TypeVar

_REPO_ROOT = Path(__file__).resolve().parents[2]
_RUNTIME_LOCK = _REPO_ROOT / ".build" / "selfhost-runtime.lock"

# How often to print wait diagnostics while blocked on the exclusive lock.
_WAIT_DIAG_INTERVAL_SEC = 30.0
# Optional hard wait timeout (seconds). 0 / unset = wait indefinitely.
_WAIT_TIMEOUT_ENV = "ARUKELLT_SELFHOST_LOCK_WAIT_SEC"

T = TypeVar("T")


def runtime_lock_path() -> Path:
    return _RUNTIME_LOCK


def _read_lock_owner_pid(lock_path: Path) -> int | None:
    try:
        text = lock_path.read_text(encoding="utf-8").strip()
    except OSError:
        return None
    if not text:
        return None
    try:
        return int(text.split()[0])
    except ValueError:
        return None


def _pid_alive(pid: int) -> bool:
    if pid <= 0:
        return False
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return False
    except PermissionError:
        return True
    return True


def _lock_wait_timeout_sec() -> float | None:
    raw = os.environ.get(_WAIT_TIMEOUT_ENV, "").strip()
    if not raw:
        return None
    try:
        value = float(raw)
    except ValueError:
        return None
    if value <= 0:
        return None
    return value


def _diag(msg: str) -> None:
    print(msg, file=sys.stderr, flush=True)


def with_selfhost_runtime_lock(fn: Callable[[], T]) -> T:
    _RUNTIME_LOCK.parent.mkdir(parents=True, exist_ok=True)
    timeout = _lock_wait_timeout_sec()
    started = time.monotonic()
    next_diag = started + _WAIT_DIAG_INTERVAL_SEC
    with _RUNTIME_LOCK.open("a+", encoding="utf-8") as lock_file:
        while True:
            try:
                fcntl.flock(lock_file.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
                break
            except BlockingIOError:
                now = time.monotonic()
                if timeout is not None and (now - started) >= timeout:
                    owner = _read_lock_owner_pid(_RUNTIME_LOCK)
                    owner_note = f" owner_pid={owner}" if owner is not None else ""
                    raise TimeoutError(
                        f"selfhost runtime lock wait exceeded {timeout:.0f}s "
                        f"({_RUNTIME_LOCK}{owner_note})"
                    ) from None
                if now >= next_diag:
                    owner = _read_lock_owner_pid(_RUNTIME_LOCK)
                    if owner is None:
                        owner_state = "owner=unknown"
                    elif _pid_alive(owner):
                        owner_state = f"owner_pid={owner} (alive)"
                    else:
                        owner_state = (
                            f"owner_pid={owner} (not running — stale lock file; "
                            "will acquire when flock is released)"
                        )
                    waited = now - started
                    _diag(
                        f"[selfhost-runtime-lock] waiting {waited:.0f}s on "
                        f"{_RUNTIME_LOCK} ({owner_state})"
                    )
                    next_diag = now + _WAIT_DIAG_INTERVAL_SEC
                time.sleep(1.0)

        # Record owner for diagnostics; flock is the authority, not this text.
        try:
            lock_file.seek(0)
            lock_file.truncate()
            lock_file.write(f"{os.getpid()}\n")
            lock_file.flush()
        except OSError:
            pass
        try:
            return fn()
        finally:
            try:
                lock_file.seek(0)
                lock_file.truncate()
                lock_file.flush()
            except OSError:
                pass
            fcntl.flock(lock_file.fileno(), fcntl.LOCK_UN)
