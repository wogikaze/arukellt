#!/usr/bin/env python3
"""Validate ADR-046 stdlib free-function eradication coverage.

Two surfaces must resolve to a class through the ordered rules in
docs/data/stdlib-api-eradication.toml:

  * every std/manifest.toml [[functions]] entry
  * every top-level `pub fn` under std/**/*.ark that has no manifest entry

Errors:
  - unclassified manifest function
  - unregistered std free (source declaration matched only by the
    [source_default] catch-all)
  - manifest entry without decl (a prelude wrapper or module-role entry
    that claims a std declaration but has none)
  - classes outside the #709 vocabulary
  - public classes without an on-disk owner issue
  - intrinsic-only entries that are actually user-reachable
  - dead rules (no manifest entry and no source declaration)
"""
from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts" / "gen"))

from stdlib_eradication import (  # noqa: E402
    CLASSES,
    classify_manifest,
    classify_source,
    intrinsic_is_non_public,
    iter_source_declarations,
    load_eradication,
    manifest_entries_without_decl,
    owner_issue_path,
    rule_label,
)

try:
    import tomllib
except ModuleNotFoundError:
    import tomli as tomllib


def load_manifest(path: Path) -> dict:
    with open(path, "rb") as handle:
        return tomllib.load(handle)


def _check_rule_owners(errors: list[str], rules: list[dict]) -> None:
    for rule in rules:
        rule_name = rule_label(rule)
        cls = rule.get("class", "")
        owner = str(rule.get("owner", ""))
        if cls == "intrinsic-only" or cls not in CLASSES:
            continue
        if not owner or owner == "none":
            errors.append(f"rule {rule_name!r}: public class needs an owner")
        elif owner_issue_path(ROOT, owner) is None:
            errors.append(f"rule {rule_name!r}: owner issue #{owner} not found")


def main() -> int:
    manifest_path = ROOT / "std" / "manifest.toml"
    data_path = ROOT / "docs" / "data" / "stdlib-api-eradication.toml"
    manifest = load_manifest(manifest_path)
    eradication = load_eradication(data_path)
    functions = manifest.get("functions", [])

    resolved, unclassified, errors = classify_manifest(eradication, manifest)
    declarations = iter_source_declarations(ROOT)
    source_resolved, source_only, source_errors = classify_source(
        eradication, manifest, declarations
    )
    errors.extend(source_errors)

    decl_names = {entry["name"] for entry in declarations}
    without_decl = manifest_entries_without_decl(manifest, decl_names)
    for entry in without_decl:
        module = entry.get("module", "(prelude)")
        kind = entry.get("kind", "(module)")
        errors.append(
            f"manifest entry without decl: {module}::{entry.get('name', '?')} "
            f"[{kind}] — add a std declaration or correct the manifest entry"
        )

    for entry in unclassified:
        name = entry.get("name", "?")
        module = entry.get("module", "(prelude)")
        errors.append(f"unclassified: {module}::{name} — add a rule or override")

    matched_rules = {id(rule) for _, rule in resolved}
    matched_rules |= {id(rule) for _, rule in source_resolved}
    for rule in eradication.get("rules", []):
        rule_name = rule_label(rule)
        if id(rule) not in matched_rules:
            errors.append(f"dead rule: {rule_name!r} matches no function or declaration")
    _check_rule_owners(errors, eradication.get("rules", []))

    for entry, rule in resolved:
        if rule.get("class") != "intrinsic-only":
            continue
        if not intrinsic_is_non_public(entry):
            errors.append(
                f"{entry.get('name')}: classified intrinsic-only but is a public "
                f"{entry.get('kind', 'symbol')} symbol"
            )

    if errors:
        print("stdlib eradication check FAILED:", file=sys.stderr)
        for err in errors:
            print(f"  x {err}", file=sys.stderr)
        print(
            f"  {len(functions)} manifest functions, {len(resolved)} classified, "
            f"{len(unclassified)} unclassified; {len(declarations)} std "
            f"declarations, {len(source_only)} unregistered, "
            f"{len(source_resolved)} classified",
            file=sys.stderr,
        )
        return 1

    manifest_counts: dict[str, int] = {}
    for _, rule in resolved:
        manifest_counts[rule["class"]] = manifest_counts.get(rule["class"], 0) + 1
    source_counts: dict[str, int] = {}
    for _, rule in source_resolved:
        source_counts[rule["class"]] = source_counts.get(rule["class"], 0) + 1
    counts = {
        cls: manifest_counts.get(cls, 0) + source_counts.get(cls, 0)
        for cls in CLASSES
    }
    summary = ", ".join(f"{cls}={counts[cls]}" for cls in sorted(counts))
    public_scope = counts["must delete"] + counts["associated-or-method"]
    print(
        f"stdlib eradication: {len(resolved)}/{len(functions)} manifest functions, "
        f"{len(source_resolved)}/{len(source_only)} unregistered std declarations "
        f"classified ({summary}); public surface={public_scope}, "
        f"intrinsic-only={counts['intrinsic-only']}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
