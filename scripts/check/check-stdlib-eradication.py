#!/usr/bin/env python3
"""Validate ADR-046 stdlib free-function eradication coverage.

Every [[functions]] entry in std/manifest.toml must resolve to a class
through the ordered rules in docs/data/stdlib-api-eradication.toml:

  - every manifest function resolves to a class (no unclassified)
  - classes are limited to the #709 vocabulary
  - non-intrinsic-only rules carry an owner issue that exists on disk
  - intrinsic-only symbols stay non-public (__intrinsic_* or intrinsic kinds)
  - every rule matches at least one manifest function (no dead rules)
"""
from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts" / "gen"))

from stdlib_eradication import (  # noqa: E402
    classify_manifest,
    load_eradication,
    owner_issue_path,
    resolve_class,
    rule_label,
)

try:
    import tomllib
except ModuleNotFoundError:
    import tomli as tomllib


def load_manifest(path: Path) -> dict:
    with open(path, "rb") as handle:
        return tomllib.load(handle)


def main() -> int:
    manifest_path = ROOT / "std" / "manifest.toml"
    data_path = ROOT / "docs" / "data" / "stdlib-api-eradication.toml"
    manifest = load_manifest(manifest_path)
    eradication = load_eradication(data_path)
    functions = manifest.get("functions", [])

    resolved, unclassified, errors = classify_manifest(eradication, manifest)

    for entry in unclassified:
        name = entry.get("name", "?")
        module = entry.get("module", "(prelude)")
        errors.append(f"unclassified: {module}::{name} — add a rule or override")

    matched_rules = {id(rule) for _, rule in resolved}
    for rule in eradication.get("rules", []):
        rule_name = rule_label(rule)
        if id(rule) not in matched_rules:
            errors.append(f"dead rule: {rule_name!r} matches no manifest function")
        cls = rule.get("class", "")
        owner = str(rule.get("owner", ""))
        if cls != "intrinsic-only":
            if not owner or owner == "none":
                errors.append(f"rule {rule_name!r}: public class needs an owner")
            elif owner_issue_path(ROOT, owner) is None:
                errors.append(f"rule {rule_name!r}: owner issue #{owner} not found")

    for entry, rule in resolved:
        cls = rule.get("class")
        if cls == "intrinsic-only":
            name = entry.get("name", "")
            kind = entry.get("kind", "")
            public = not name.startswith("__intrinsic_") and kind not in (
                "intrinsic",
                "intrinsic_wrapper",
            )
            if public:
                errors.append(
                    f"{name}: classified intrinsic-only but is a public {kind} symbol",
                )

    if errors:
        print("stdlib eradication check FAILED:", file=sys.stderr)
        for err in errors:
            print(f"  x {err}", file=sys.stderr)
        print(
            f"  {len(functions)} manifest functions, {len(resolved)} classified, "
            f"{len(unclassified)} unclassified",
            file=sys.stderr,
        )
        return 1

    counts = {}
    for _, rule in resolved:
        counts[rule["class"]] = counts.get(rule["class"], 0) + 1
    summary = ", ".join(f"{cls}={counts.get(cls, 0)}" for cls in sorted(counts))
    print(f"stdlib eradication: {len(resolved)}/{len(functions)} classified ({summary})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
