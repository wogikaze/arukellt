"""Serialize pinned-bootstrap wasmtime/selfhost compile usage across verify gates."""

from __future__ import annotations

import fcntl
import importlib.util
import os
import sys
import time
from collections.abc import Callable
from pathlib import Path
from typing import TypeVar

# How often to print wait diagnostics while blocked on the exclusive lock.
_WAIT_DIAG_INTERVAL_SEC = 30.0
# Optional hard wait timeout (seconds). 0 / unset = wait indefinitely.
_WAIT_TIMEOUT_ENV = "ARUKELLT_SELFHOST_LOCK_WAIT_SEC"

T = TypeVar("T")


def _load_build_paths():
    lib = Path(__file__).resolve().parents[1] / "lib" / "build_paths.py"
    spec = importlib.util.spec_from_file_location("arukellt_build_paths", lib)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"missing {lib}")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def _default_root() -> Path:
    # scripts/selfhost -> repo root (worktree-aware via this file's location)
    return Path(__file__).resolve().parents[2]


def runtime_lock_path(root: Path | None = None) -> Path:
    build_paths = _load_build_paths()
    return build_paths.runtime_lock_path(root if root is not None else _default_root())


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


def with_selfhost_runtime_lock(fn: Callable[[], T], root: Path | None = None) -> T:
    """Run ``fn`` under an exclusive flock on the worktree/build-local lock."""
    lock_path = runtime_lock_path(root)
    lock_path.parent.mkdir(parents=True, exist_ok=True)
    timeout = _lock_wait_timeout_sec()
    started = time.monotonic()
    next_diag = started + _WAIT_DIAG_INTERVAL_SEC
    with lock_path.open("a+", encoding="utf-8") as lock_file:
        while True:
            try:
                fcntl.flock(lock_file.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
                break
            except BlockingIOError:
                now = time.monotonic()
                if timeout is not None and (now - started) >= timeout:
                    owner = _read_lock_owner_pid(lock_path)
                    owner_note = f" owner_pid={owner}" if owner is not None else ""
                    raise TimeoutError(
                        f"selfhost runtime lock wait exceeded {timeout:.0f}s "
                        f"({lock_path}{owner_note})"
                    ) from None
                if now >= next_diag:
                    owner = _read_lock_owner_pid(lock_path)
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
                        f"{lock_path} ({owner_state})"
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
