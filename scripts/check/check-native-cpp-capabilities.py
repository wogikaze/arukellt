#!/usr/bin/env python3
"""Validate the native-cpp MIR opcode and CoreOp capability registry."""

from __future__ import annotations

import argparse
import ast
import json
import re
import sys
from collections import Counter
from pathlib import Path
from typing import NamedTuple


REPO_ROOT = Path(__file__).resolve().parents[2]
MIR_OPCODE_PATTERN = re.compile(r"\bfn\s+(MIR_[A-Z0-9_]+)\s*\(\s*\)\s*->\s*i32")
ALLOWED_STATUSES = {"supported", "planned", "unsupported"}
REQUIRED_FIELDS = {
    "supported": ("implementation",),
    "planned": ("phase", "implementation"),
    "unsupported": ("reason",),
}
GENERATED_RELATIVE_PATH = Path("src/compiler/native_c_capabilities_generated.ark")


class ValidationSummary(NamedTuple):
    mir_opcode_count: int
    core_op_count: int
    status_counts: dict[str, int]


def _parse_scalar(raw_value: str, path: Path, line_number: int) -> object:
    value = raw_value.split("#", 1)[0].strip()
    if not value:
        raise ValueError(f"{path}:{line_number}: empty value")
    if value.startswith('"') or value.startswith("["):
        try:
            return ast.literal_eval(value)
        except (SyntaxError, ValueError) as error:
            raise ValueError(f"{path}:{line_number}: invalid string") from error
    try:
        return int(value)
    except ValueError:
        return value


def _parse_metadata(path: Path) -> tuple[dict[str, object], dict[str, object]]:
    top_level: dict[str, object] = {}
    status_schema: dict[str, object] = {}
    current = top_level
    for line_number, raw_line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        line = raw_line.strip()
        if not line or line.startswith("#"):
            continue
        if line == "[status_schema]":
            current = status_schema
            continue
        if line.startswith("["):
            current = {}
            continue
        if "=" not in line:
            continue
        key, raw_value = line.split("=", 1)
        current[key.strip()] = _parse_scalar(raw_value, path, line_number)
    return top_level, status_schema


def _parse_array_tables(path: Path, table_names: set[str]) -> dict[str, list[dict[str, object]]]:
    tables = {name: [] for name in table_names}
    current: dict[str, object] | None = None
    for line_number, raw_line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        line = raw_line.strip()
        if not line or line.startswith("#"):
            continue
        array_match = re.fullmatch(r"\[\[([A-Za-z0-9_]+)\]\]", line)
        if array_match:
            current = None
            table_name = array_match.group(1)
            if table_name in tables:
                current = {}
                tables[table_name].append(current)
            continue
        if line.startswith("["):
            current = None
            continue
        if current is None or "=" not in line:
            continue
        key, raw_value = line.split("=", 1)
        current[key.strip()] = _parse_scalar(raw_value, path, line_number)
    return tables


def _source_mir_opcodes(root: Path) -> list[str]:
    return MIR_OPCODE_PATTERN.findall((root / "src/compiler/mir/opcodes.ark").read_text(encoding="utf-8"))


def _source_core_ops(root: Path) -> list[str]:
    operations = _parse_array_tables(root / "data/core-ops.toml", {"operations"})["operations"]
    return [str(entry.get("id", "")) for entry in operations]


def _validate_identity_set(label: str, source_ids: list[str], entries: list[dict[str, object]]) -> list[str]:
    registry_ids = [str(entry.get("id", "")) for entry in entries]
    source_set = set(source_ids)
    registry_set = set(registry_ids)
    errors: list[str] = []
    missing = sorted(source_set - registry_set)
    unknown = sorted(registry_set - source_set)
    duplicates = sorted(identifier for identifier, count in Counter(registry_ids).items() if count > 1)
    if missing:
        errors.append(f"missing {label}: {', '.join(missing)}")
    if unknown:
        errors.append(f"unknown {label}: {', '.join(unknown)}")
    if duplicates:
        errors.append(f"duplicate {label}: {', '.join(duplicates)}")
    return errors


def _validate_statuses(entries: list[dict[str, object]], label: str) -> list[str]:
    errors: list[str] = []
    for entry in entries:
        identifier = str(entry.get("id", "<missing-id>"))
        status = str(entry.get("status", ""))
        if status not in ALLOWED_STATUSES:
            errors.append(f"{label} {identifier}: unknown status `{status}`")
            continue
        for field in REQUIRED_FIELDS[status]:
            if entry.get(field) in (None, ""):
                errors.append(f"{label} {identifier}: status `{status}` requires `{field}`")
        if status == "planned" and (not isinstance(entry.get("phase"), int) or int(entry["phase"]) < 0):
            errors.append(f"{label} {identifier}: `phase` must be a non-negative integer")
    return errors


def _validate_schema(path: Path) -> list[str]:
    top_level, status_schema = _parse_metadata(path)
    errors: list[str] = []
    if top_level.get("schema_version") != 1:
        errors.append("schema_version must be 1")
    if top_level.get("target") != "native-cpp":
        errors.append("target must be `native-cpp`")
    if status_schema.get("allowed") != ["supported", "planned", "unsupported"]:
        errors.append("status_schema.allowed does not match the native-cpp capability schema")
    for status, fields in REQUIRED_FIELDS.items():
        key = f"{status}_requires"
        if status_schema.get(key) != list(fields):
            errors.append(f"status_schema.{key} does not match required fields")
    return errors


def _status_detail(entry: dict[str, object]) -> str:
    if entry["status"] == "supported":
        return "supported"
    if entry["status"] == "planned":
        return f"planned phase {entry['phase']}"
    return f"unsupported: {entry['reason']}"


def render_generated_ark(entries: list[dict[str, object]]) -> str:
    lines = [
        "// Generated from data/native-cpp-capabilities.toml.",
        "// Run scripts/check/check-native-cpp-capabilities.py --write-generated.",
        "",
        "use mir::opcodes",
        "",
        "fn native_c_capability_opcode_name(op: i32) -> String {",
    ]
    for entry in entries:
        identifier = str(entry["id"])
        lines.extend([f"    if op == opcodes::{identifier}() {{", f"        return String_from({json.dumps(identifier)})", "    }"])
    lines.extend(['    concat(String_from("MIR_OPCODE_"), i32_to_string(op))', "}", "", "fn native_c_capability_status_detail(op: i32) -> String {"])
    for entry in entries:
        identifier = str(entry["id"])
        lines.extend([f"    if op == opcodes::{identifier}() {{", f"        return String_from({json.dumps(_status_detail(entry))})", "    }"])
    lines.extend(['    String_from("unknown capability")', "}", "", "fn native_c_capability_is_supported(op: i32) -> bool {"])
    supported = [str(entry["id"]) for entry in entries if entry["status"] == "supported"]
    for index, identifier in enumerate(supported):
        lines.append(f"    op == opcodes::{identifier}()" + (" ||" if index < len(supported) - 1 else ""))
    if not supported:
        lines.append("    false")
    lines.extend(["}", ""])
    return "\n".join(lines)


def validate_repository(root: Path = REPO_ROOT) -> ValidationSummary:
    registry_path = root / "data/native-cpp-capabilities.toml"
    tables = _parse_array_tables(registry_path, {"mir_opcodes", "core_ops"})
    mir_entries = tables["mir_opcodes"]
    core_entries = tables["core_ops"]
    source_mir = _source_mir_opcodes(root)
    source_core = _source_core_ops(root)
    errors = _validate_schema(registry_path)
    errors.extend(_validate_identity_set("MIR opcode", source_mir, mir_entries))
    errors.extend(_validate_identity_set("CoreOp", source_core, core_entries))
    errors.extend(_validate_statuses(mir_entries, "MIR opcode"))
    errors.extend(_validate_statuses(core_entries, "CoreOp"))
    if errors:
        raise ValueError("\n".join(errors))
    generated_path = root / GENERATED_RELATIVE_PATH
    if generated_path.is_file() and generated_path.read_text(encoding="utf-8") != render_generated_ark(mir_entries):
        raise ValueError(f"generated capability view is stale: {GENERATED_RELATIVE_PATH}; run with --write-generated")
    counts = Counter(str(entry["status"]) for entry in mir_entries + core_entries)
    return ValidationSummary(len(source_mir), len(source_core), {status: counts[status] for status in sorted(ALLOWED_STATUSES)})


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=REPO_ROOT)
    parser.add_argument("--write-generated", action="store_true")
    args = parser.parse_args(argv)
    root = args.root.resolve()
    if args.write_generated:
        entries = _parse_array_tables(root / "data/native-cpp-capabilities.toml", {"mir_opcodes"})["mir_opcodes"]
        generated_path = root / GENERATED_RELATIVE_PATH
        generated_path.parent.mkdir(parents=True, exist_ok=True)
        generated_path.write_text(render_generated_ark(entries), encoding="utf-8")
    try:
        summary = validate_repository(root)
    except (OSError, ValueError) as error:
        print(f"native-cpp capability validation failed: {error}", file=sys.stderr)
        return 1
    counts = summary.status_counts
    print(
        f"native-cpp capabilities: MIR opcodes={summary.mir_opcode_count} CoreOps={summary.core_op_count} "
        f"supported={counts['supported']} planned={counts['planned']} unsupported={counts['unsupported']}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
