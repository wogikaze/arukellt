#!/usr/bin/env python3
"""Collect the native-cpp selfhost MIR coverage receipt."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import re
import sys
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
START_MARKER = "NATIVE_CPP_COVERAGE_V1"
END_MARKER = "NATIVE_CPP_COVERAGE_END"
OPCODE_PATTERN = re.compile(
    r"\bfn\s+(MIR_[A-Z0-9_]+)\s*\(\s*\)\s*->\s*i32\s*\{\s*([0-9]+)\s*\}"
)
CORE_OP_ID_PATTERN = re.compile(r'^id\s*=\s*"([^"]+)"')


def core_op_runtime_ids(source: str) -> list[str]:
    """Return CoreOp IDs in the generated registry's canonical sorted order."""
    operation_ids: list[str] = []
    in_operation = False
    for raw_line in source.splitlines():
        line = raw_line.strip()
        if line == "[[operations]]":
            in_operation = True
            continue
        if line.startswith("[[") or (line.startswith("[") and not line.startswith("[[")):
            in_operation = False
            continue
        if not in_operation:
            continue
        match = CORE_OP_ID_PATTERN.match(line)
        if match:
            operation_ids.append(match.group(1))
            in_operation = False
    return sorted(operation_ids)


def parse_coverage_report(output: str) -> dict[str, object]:
    start = output.find(START_MARKER)
    end = output.find(END_MARKER, start + len(START_MARKER))
    if start < 0 or end < 0:
        raise ValueError("native-cpp coverage markers were not emitted")

    summary: dict[str, int] = {}
    opcodes: dict[int, int] = {}
    core_ops: dict[int, int] = {}
    legacy_types: dict[int, int] = {}
    types: list[dict[str, object]] = []
    host_functions: list[dict[str, object]] = []
    unresolved_calls: list[dict[str, str]] = []
    body = output[start + len(START_MARKER):end]
    for raw_line in body.splitlines():
        line = raw_line.strip()
        if not line:
            continue
        parts = line.split("|")
        category = parts[0]
        try:
            if category == "summary" and len(parts) == 3:
                summary[parts[1]] = int(parts[2])
            elif category == "opcode" and len(parts) == 3:
                opcodes[int(parts[1])] = int(parts[2])
            elif category == "core_op" and len(parts) == 3:
                core_ops[int(parts[1])] = int(parts[2])
            elif category == "legacy_type" and len(parts) == 3:
                legacy_types[int(parts[1])] = int(parts[2])
            elif category == "type_entry" and len(parts) == 6:
                types.append(
                    {
                        "type_id": int(parts[1]),
                        "kind": int(parts[2]),
                        "use_count": int(parts[3]),
                        "type_parameter_count": int(parts[4]),
                        "name": parts[5],
                    }
                )
            elif category == "host_function" and len(parts) == 4:
                host_functions.append(
                    {"function_id": int(parts[1]), "count": int(parts[2]), "name": parts[3]}
                )
            elif category == "unresolved_call" and len(parts) == 3:
                unresolved_calls.append({"function": parts[1], "callee": parts[2]})
            else:
                raise ValueError
        except ValueError as error:
            raise ValueError(f"malformed native-cpp coverage line: {line}") from error
    return {
        "summary": summary,
        "opcodes": opcodes,
        "core_ops": core_ops,
        "legacy_types": legacy_types,
        "types": types,
        "host_functions": host_functions,
        "unresolved_calls": unresolved_calls,
    }


def build_receipt(root: Path, raw: dict[str, object], compiler: Path) -> dict[str, object]:
    opcode_source = (root / "src/compiler/mir/opcodes.ark").read_text(encoding="utf-8")
    opcode_names = {int(raw_id): name for name, raw_id in OPCODE_PATTERN.findall(opcode_source)}
    core_op_source = (root / "data/core-ops.toml").read_text(encoding="utf-8")
    core_op_names = core_op_runtime_ids(core_op_source)
    raw_opcodes = raw["opcodes"]
    raw_core_ops = raw["core_ops"]
    assert isinstance(raw_opcodes, dict)
    assert isinstance(raw_core_ops, dict)
    unknown_opcodes = sorted(set(raw_opcodes) - set(opcode_names))
    unknown_core_ops = sorted(index for index in raw_core_ops if index >= len(core_op_names))
    if unknown_opcodes or unknown_core_ops:
        raise ValueError(
            f"coverage contains unknown IDs: opcodes={unknown_opcodes} core_ops={unknown_core_ops}"
        )

    return {
        "schema_version": 1,
        "target": "native-cpp",
        "compiler_sha256": hashlib.sha256(compiler.read_bytes()).hexdigest(),
        "summary": raw["summary"],
        "mir_opcodes": [
            {"raw_id": raw_id, "id": opcode_names[raw_id], "count": raw_opcodes[raw_id]}
            for raw_id in sorted(raw_opcodes)
        ],
        "core_ops": [
            {"raw_id": raw_id, "id": core_op_names[raw_id], "count": raw_core_ops[raw_id]}
            for raw_id in sorted(raw_core_ops)
        ],
        "legacy_value_types": [
            {"raw_id": raw_id, "count": count}
            for raw_id, count in sorted(raw["legacy_types"].items())
        ],
        "type_entries": raw["types"],
        "host_functions": raw["host_functions"],
        "unresolved_calls": raw["unresolved_calls"],
    }


def load_selfhost_checks(root: Path):
    path = root / "scripts/selfhost/checks.py"
    spec = importlib.util.spec_from_file_location("arukellt_selfhost_checks", path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=REPO_ROOT)
    parser.add_argument("--compiler", type=Path)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args(argv)
    root = args.root.resolve()
    checks = load_selfhost_checks(root)
    compiler = args.compiler or checks._selfhost_dir(root) / "arukellt-s2-runtime.wasm"
    output = args.output or checks._build_dir(root) / "selfhost/native-cpp/coverage-receipt.json"
    if not compiler.is_file():
        print(f"native-cpp coverage failed: compiler not found: {compiler}", file=sys.stderr)
        return 1

    result = checks._wasm_compile_selfhost_source(
        checks._find_wasmtime(),
        compiler.resolve(),
        ".build/selfhost/native-cpp/coverage-probe.c",
        root,
        target="native-cpp",
        wasi_version="wasi-p1",
        extra_args=["--dump-phases", "native-cpp"],
    )
    combined = (result.stdout or "") + "\n" + (result.stderr or "")
    try:
        receipt = build_receipt(root, parse_coverage_report(combined), compiler.resolve())
    except (OSError, ValueError) as error:
        print(f"native-cpp coverage failed: {error}", file=sys.stderr)
        return 1
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    summary = receipt["summary"]
    print(
        "native-cpp coverage: "
        f"functions={summary['functions']} instructions={summary['instructions']} "
        f"opcodes={len(receipt['mir_opcodes'])} CoreOps={len(receipt['core_ops'])} "
        f"receipt={output}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
