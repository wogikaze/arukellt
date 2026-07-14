"""Typed read/write model for the shared Ark code-quality baseline."""

from __future__ import annotations

import json
from dataclasses import dataclass, replace
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:  # Python 3.10 on supported development hosts
    import tomli as tomllib  # type: ignore[no-redef]


INVENTORY_COUNT_KEYS = (
    "tabs_files",
    "extreme_indent_lines",
    "lines_ge_200",
    "thin_wrappers",
    "single_function_files",
)
DISTRIBUTION_KEYS = ("count", "p50", "p75", "p90", "p95", "max")


class BaselineError(ValueError):
    """A baseline cannot be parsed or does not satisfy its schema."""


@dataclass(frozen=True)
class InventoryMetadata:
    owner: str
    increase_requires_tracking_issue: bool
    last_update_issue: int
    max_leading_ws: int
    max_line_len_hard: int


@dataclass(frozen=True)
class MetricsMetadata:
    issue: int
    reason: str


@dataclass(frozen=True)
class QualityBaseline:
    inventory: InventoryMetadata
    counts: dict[str, int]
    metrics_metadata: MetricsMetadata
    metrics: dict[str, dict[str, int | float]]


def _table(data: dict, key: str) -> dict:
    value = data.get(key)
    if not isinstance(value, dict):
        raise BaselineError(f"baseline section [{key}] is missing or is not a table")
    return value


def _required_value(table: dict, section: str, key: str, expected: type):
    if key not in table:
        raise BaselineError(f"baseline [{section}] missing required key: {key}")
    value = table[key]
    if expected is int:
        valid = isinstance(value, int) and not isinstance(value, bool)
    else:
        valid = isinstance(value, expected)
    if not valid:
        raise BaselineError(
            f"baseline [{section}].{key} must be {expected.__name__}, "
            f"got {type(value).__name__}"
        )
    return value


def read_baseline(path: Path, metric_names: tuple[str, ...]) -> QualityBaseline:
    try:
        data = tomllib.loads(path.read_text(encoding="utf-8"))
    except OSError as exc:
        raise BaselineError(f"cannot read baseline {path}: {exc}") from exc
    except tomllib.TOMLDecodeError as exc:
        raise BaselineError(f"malformed baseline TOML {path}: {exc}") from exc

    inventory_table = _table(data, "inventory")
    inventory = InventoryMetadata(
        owner=_required_value(inventory_table, "inventory", "owner", str),
        increase_requires_tracking_issue=_required_value(
            inventory_table,
            "inventory",
            "increase_requires_tracking_issue",
            bool,
        ),
        last_update_issue=_required_value(
            inventory_table, "inventory", "last_update_issue", int
        ),
        max_leading_ws=_required_value(
            inventory_table, "inventory", "max_leading_ws", int
        ),
        max_line_len_hard=_required_value(
            inventory_table, "inventory", "max_line_len_hard", int
        ),
    )

    counts_table = _table(data, "counts")
    counts = {
        key: _required_value(counts_table, "counts", key, int)
        for key in INVENTORY_COUNT_KEYS
    }
    for key, value in counts.items():
        if value < 0:
            raise BaselineError(f"baseline [counts].{key} must be non-negative")

    metadata_table = _table(data, "metrics_metadata")
    metrics_metadata = MetricsMetadata(
        issue=_required_value(metadata_table, "metrics_metadata", "issue", int),
        reason=_required_value(metadata_table, "metrics_metadata", "reason", str),
    )

    metrics_table = _table(data, "metrics")
    metrics: dict[str, dict[str, int | float]] = {}
    for metric_name in metric_names:
        distribution = metrics_table.get(metric_name)
        if not isinstance(distribution, dict):
            raise BaselineError(
                f"baseline section [metrics.{metric_name}] is missing or is not a table"
            )
        parsed: dict[str, int | float] = {}
        for key in DISTRIBUTION_KEYS:
            if key not in distribution:
                raise BaselineError(
                    f"baseline [metrics.{metric_name}] missing required key: {key}"
                )
            value = distribution[key]
            if isinstance(value, bool) or not isinstance(value, (int, float)):
                raise BaselineError(
                    f"baseline [metrics.{metric_name}].{key} must be numeric, "
                    f"got {type(value).__name__}"
                )
            if value < 0:
                raise BaselineError(
                    f"baseline [metrics.{metric_name}].{key} must be non-negative"
                )
            parsed[key] = value
        metrics[metric_name] = parsed
    return QualityBaseline(inventory, counts, metrics_metadata, metrics)


def _number(value: int | float) -> str:
    if isinstance(value, int) or float(value).is_integer():
        return str(int(value))
    return f"{float(value):.6f}".rstrip("0").rstrip(".")


def render_baseline(baseline: QualityBaseline, metric_names: tuple[str, ...]) -> str:
    inventory = baseline.inventory
    lines = [
        "# Shared Ark compiler inventory and advisory metrics baseline.",
        "# Update only through the canonical inventory or metrics writer.",
        "",
        "[inventory]",
        f"owner = {json.dumps(inventory.owner, ensure_ascii=False)}",
        "increase_requires_tracking_issue = "
        + str(inventory.increase_requires_tracking_issue).lower(),
        f"last_update_issue = {inventory.last_update_issue}",
        f"max_leading_ws = {inventory.max_leading_ws}",
        f"max_line_len_hard = {inventory.max_line_len_hard}",
        "",
        "[counts]",
    ]
    for key in INVENTORY_COUNT_KEYS:
        lines.append(f"{key} = {baseline.counts[key]}")
    lines.extend(
        (
            "",
            "[metrics_metadata]",
            f"issue = {baseline.metrics_metadata.issue}",
            "reason = "
            + json.dumps(baseline.metrics_metadata.reason, ensure_ascii=False),
        )
    )
    for metric_name in metric_names:
        lines.extend(("", f"[metrics.{metric_name}]"))
        for key in DISTRIBUTION_KEYS:
            lines.append(f"{key} = {_number(baseline.metrics[metric_name][key])}")
    return "\n".join(lines) + "\n"


def write_baseline(
    path: Path, baseline: QualityBaseline, metric_names: tuple[str, ...]
) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(render_baseline(baseline, metric_names), encoding="utf-8")


def with_inventory(
    baseline: QualityBaseline,
    counts: dict[str, int],
    issue: int,
) -> QualityBaseline:
    increases = [
        key for key in INVENTORY_COUNT_KEYS if counts[key] > baseline.counts[key]
    ]
    if increases:
        raise BaselineError(
            "inventory baseline update may only lower counts; increases: "
            + ", ".join(increases)
        )
    return replace(
        baseline,
        inventory=replace(baseline.inventory, last_update_issue=issue),
        counts={key: counts[key] for key in INVENTORY_COUNT_KEYS},
    )


def with_metrics(
    baseline: QualityBaseline,
    metrics: dict[str, dict[str, int | float]],
    issue: int,
    reason: str,
) -> QualityBaseline:
    return replace(
        baseline,
        metrics_metadata=MetricsMetadata(issue, reason),
        metrics=metrics,
    )
