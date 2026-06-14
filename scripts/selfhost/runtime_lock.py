"""Serialize pinned-bootstrap wasmtime/selfhost compile usage across verify gates."""

from __future__ import annotations

import fcntl
from collections.abc import Callable
from pathlib import Path
from typing import TypeVar

_REPO_ROOT = Path(__file__).resolve().parents[2]
_RUNTIME_LOCK = _REPO_ROOT / ".build" / "selfhost-runtime.lock"

T = TypeVar("T")


def with_selfhost_runtime_lock(fn: Callable[[], T]) -> T:
    _RUNTIME_LOCK.parent.mkdir(parents=True, exist_ok=True)
    with _RUNTIME_LOCK.open("w", encoding="utf-8") as lock_file:
        fcntl.flock(lock_file.fileno(), fcntl.LOCK_EX)
        return fn()
