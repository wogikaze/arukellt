#!/usr/bin/env python3
"""Compare MIR function multisets from native vs wasmtime compile dumps (#832)."""

from __future__ import annotations

import argparse
import json
import re
import sys
from collections import Counter
from pathlib import Path

FN_RE = re.compile(r"^\s*fn\s+(\S+)")
REACH_RE = re.compile(
    r"lower\.reachability_fns:\s*before=(\d+)\s+after=(\d+)"
)
MONO_RE = re.compile(r"summary\|mono_instances\|(\d+)")
MIR_MODULE_RE = re.compile(r"^MIR module", re.IGNORECASE)


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8", errors="replace")


def extract_fn_names(text: str) -> list[str]:
    names: list[str] = []
    for line in text.splitlines():
        match = FN_RE.match(line.strip())
        if match:
            names.append(match.group(1))
    return names


def extract_reachability(text: str) -> tuple[int | None, int | None]:
    match = REACH_RE.search(text)
    if not match:
        return None, None
    return int(match.group(1)), int(match.group(2))


def extract_mono(text: str) -> int | None:
    match = MONO_RE.search(text)
    return int(match.group(1)) if match else None


def counter_diff(left: Counter[str], right: Counter[str]) -> tuple[Counter[str], Counter[str]]:
    only_left: Counter[str] = Counter()
    only_right: Counter[str] = Counter()
    for name in sorted(set(left) | set(right)):
        delta = left[name] - right[name]
        if delta > 0:
            only_left[name] = delta
        elif delta < 0:
            only_right[name] = -delta
    return only_left, only_right


def first_ordered_divergence(
    left: list[str], right: list[str]
) -> dict[str, object] | None:
    limit = min(len(left), len(right))
    for index in range(limit):
        if left[index] != right[index]:
            start = max(0, index - 5)
            end = min(max(len(left), len(right)), index + 6)
            return {
                "index": index,
                "native": left[index],
                "wasmtime": right[index],
                "native_window": left[start:end],
                "wasmtime_window": right[start:end],
            }
    if len(left) != len(right):
        index = limit
        return {
            "index": index,
            "native": left[index] if index < len(left) else None,
            "wasmtime": right[index] if index < len(right) else None,
            "native_window": left[max(0, index - 5) : index + 6],
            "wasmtime_window": right[max(0, index - 5) : index + 6],
        }
    return None


def short_name(name: str) -> str:
    if "::" in name:
        return name.rsplit("::", 1)[-1]
    if "__" in name:
        return name.rsplit("__", 1)[-1]
    return name


def classify(name: str) -> str:
    lowered = name.lower()
    if "$mono$" in lowered or "mono$" in lowered or ".mono." in lowered:
        return "mono"
    if "closure" in lowered or "$closure" in lowered:
        return "closure"
    if "wrapper" in lowered or "adapter" in lowered:
        return "wrapper"
    if "__" in name or "::" in name:
        return "normal"
    return "fallback_candidate"


def load_side(stdout: Path, stderr: Path) -> dict[str, object]:
    out = read_text(stdout) if stdout.exists() else ""
    err = read_text(stderr) if stderr.exists() else ""
    combined = out + "\n" + err
    names = extract_fn_names(combined)
    before, after = extract_reachability(combined)
    return {
        "fn_names": names,
        "fn_count": len(names),
        "reachability_before": before,
        "reachability_after": after,
        "mono_instances": extract_mono(combined),
        "has_mir_module_banner": bool(MIR_MODULE_RE.search(combined)),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--native-stdout", type=Path, required=True)
    parser.add_argument("--native-stderr", type=Path, required=True)
    parser.add_argument("--wasmtime-stdout", type=Path, required=True)
    parser.add_argument("--wasmtime-stderr", type=Path, required=True)
    parser.add_argument("--json-out", type=Path, required=True)
    args = parser.parse_args()

    native = load_side(args.native_stdout, args.native_stderr)
    wasmtime = load_side(args.wasmtime_stdout, args.wasmtime_stderr)

    native_names: list[str] = native["fn_names"]  # type: ignore[assignment]
    wasmtime_names: list[str] = wasmtime["fn_names"]  # type: ignore[assignment]
    native_counter = Counter(native_names)
    wasmtime_counter = Counter(wasmtime_names)
    only_native, only_wasmtime = counter_diff(native_counter, wasmtime_counter)

    native_only_rows = []
    for name, count in only_native.most_common():
        native_only_rows.append(
            {
                "name": name,
                "native_count": native_counter[name],
                "wasmtime_count": wasmtime_counter[name],
                "delta": count,
                "kind": classify(name),
                "short_name": short_name(name),
            }
        )

    wasmtime_only_rows = []
    for name, count in only_wasmtime.most_common():
        wasmtime_only_rows.append(
            {
                "name": name,
                "native_count": native_counter[name],
                "wasmtime_count": wasmtime_counter[name],
                "delta": count,
                "kind": classify(name),
                "short_name": short_name(name),
            }
        )

    short_groups: dict[str, list[str]] = {}
    for row in native_only_rows:
        short_groups.setdefault(row["short_name"], []).append(row["name"])

    receipt = {
        "native": {
            "fn_count": native["fn_count"],
            "reachability_before": native["reachability_before"],
            "reachability_after": native["reachability_after"],
            "mono_instances": native["mono_instances"],
        },
        "wasmtime": {
            "fn_count": wasmtime["fn_count"],
            "reachability_before": wasmtime["reachability_before"],
            "reachability_after": wasmtime["reachability_after"],
            "mono_instances": wasmtime["mono_instances"],
        },
        "difference": (native["fn_count"] or 0) - (wasmtime["fn_count"] or 0),
        "native_only": native_only_rows,
        "wasmtime_only": wasmtime_only_rows,
        "native_only_total": sum(only_native.values()),
        "wasmtime_only_total": sum(only_wasmtime.values()),
        "short_name_groups_in_native_only": {
            short: names for short, names in short_groups.items() if len(names) > 1
        },
        "first_ordered_divergence": first_ordered_divergence(
            native_names, wasmtime_names
        ),
    }

    args.json_out.parent.mkdir(parents=True, exist_ok=True)
    args.json_out.write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n")

    print("native reachability:  {} -> {}".format(
        native["reachability_before"], native["reachability_after"]
    ))
    print("wasmtime reachability: {} -> {}".format(
        wasmtime["reachability_before"], wasmtime["reachability_after"]
    ))
    print("native MIR fns:   {}".format(native["fn_count"]))
    print("wasmtime MIR fns: {}".format(wasmtime["fn_count"]))
    print("difference:       {}".format(receipt["difference"]))
    print("mono native/wasmtime: {} / {}".format(
        native["mono_instances"], wasmtime["mono_instances"]
    ))
    print()
    print("native-only:")
    if not native_only_rows:
        print("  none")
    else:
        for row in native_only_rows:
            print("  {:>3}  {}  ({})".format(row["delta"], row["name"], row["kind"]))
    print()
    print("wasmtime-only:")
    if not wasmtime_only_rows:
        print("  none")
    else:
        for row in wasmtime_only_rows:
            print("  {:>3}  {}  ({})".format(row["delta"], row["name"], row["kind"]))
    print()
    divergence = receipt["first_ordered_divergence"]
    print("first ordered divergence:")
    if divergence is None:
        print("  none (ordered lists equal)")
    else:
        print("  index {}".format(divergence["index"]))
        print("  native:   {}".format(divergence["native"]))
        print("  wasmtime: {}".format(divergence["wasmtime"]))
    print()
    print("wrote {}".format(args.json_out))
    return 0


if __name__ == "__main__":
    sys.exit(main())
