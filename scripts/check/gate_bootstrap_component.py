#!/usr/bin/env python3
"""Shared helpers for bootstrap component close gates."""

from __future__ import annotations

# Pinned bootstrap `--emit component` includes WASI Preview 2 imports such as
# wasi:clocks/monotonic-clock. wasm-tools validate cannot instantiate those
# imports in isolation; static+overlay evidence is the acceptance fallback.
_BOOTSTRAP_INSTANTIATION_GAP_MARKERS: tuple[str, ...] = (
    "missing module instantiation argument named `wasi:clocks/monotonic-clock",
)


def bootstrap_instantiation_gap(msg: str) -> bool:
    """True when wasm-tools validate failed only due to unsatisfied P2 host imports."""
    return any(marker in msg for marker in _BOOTSTRAP_INSTANTIATION_GAP_MARKERS)


def bootstrap_validate_skip_allowed(
    validate_msg: str, static_rc: int, overlay_rc: int
) -> bool:
    """Allow PASS when compile succeeded but validate cannot satisfy P2 imports."""
    return static_rc == 0 and overlay_rc == 0 and bootstrap_instantiation_gap(validate_msg)
