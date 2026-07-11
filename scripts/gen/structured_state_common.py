"""Shared derivations for structured documentation state.

Views must import these functions rather than reinterpreting evidence fields.
"""

from __future__ import annotations

from datetime import date, datetime


def check_freshness(check: dict, *, as_of: date | None = None) -> str:
    """Derive evidence age independently from the observed result."""
    verified_at = check.get("verified_at")
    stale_after_days = check.get("stale_after_days")
    if not verified_at or stale_after_days is None:
        return "unknown"
    verified = datetime.strptime(verified_at, "%Y-%m-%d").date()
    today = as_of or date.today()
    return "stale" if (today - verified).days > int(stale_after_days) else "fresh"
