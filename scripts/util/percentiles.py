"""Small, explicit percentile helpers shared by benchmark receipts."""

from __future__ import annotations

from collections.abc import Sequence


def percentile_linear(samples: Sequence[float], percentile: float) -> float | None:
    """Return a percentile using linear interpolation over ``[0, n - 1]``.

    Keeping the interpolation rule here makes small-sample receipts reproducible;
    in particular, p95 for ten samples is not silently rounded down to the ninth
    sorted sample.
    """
    if not samples:
        return None
    if not 0.0 <= percentile <= 100.0:
        raise ValueError(f"percentile must be between 0 and 100: {percentile}")
    ordered = sorted(float(sample) for sample in samples)
    position = (len(ordered) - 1) * percentile / 100.0
    lower = int(position)
    upper = min(lower + 1, len(ordered) - 1)
    fraction = position - lower
    return ordered[lower] * (1.0 - fraction) + ordered[upper] * fraction
