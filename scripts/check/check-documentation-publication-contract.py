#!/usr/bin/env python3
"""Strict publication contract derived from the 2026-07-11 documentation audit."""

from __future__ import annotations

import re
import sys
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:
    import tomli as tomllib

ROOT = Path(__file__).resolve().parents[2]


def load(path: str) -> dict:
    return tomllib.loads((ROOT / path).read_text(encoding="utf-8"))


def main() -> int:
    failures: list[str] = []
    manifest = load("std/manifest.toml")
    project = load("docs/data/project-state.toml")
    cli = load("docs/data/cli-surface.toml")
    release = load("docs/data/release-guarantees.toml")

    expected = ["stable", "provisional", "experimental", "deprecated"]
    if project.get("stdlib_api_lifecycle", {}).get("labels") != expected:
        failures.append("project-state public API lifecycle differs from ADR-014")
    if any(fn.get("stability") == "unimplemented" for fn in manifest.get("functions", [])):
        failures.append("public stdlib manifest contains unimplemented lifecycle")

    maturity = (ROOT / "docs/language/maturity-matrix.md").read_text(encoding="utf-8")
    if "Standard Library API" in maturity or "std::host::" in maturity:
        failures.append("language maturity matrix independently classifies stdlib APIs")

    process = (ROOT / "docs/stdlib/modules/process.md").read_text(encoding="utf-8")
    if "including the program name" in process or "including argv[0]" in process:
        failures.append("argv[0] contract contradicts the tested exclusion contract")
    if "Target/profile availability: mixed" not in process:
        failures.append("mixed process/env page lacks mixed availability axis")

    reference = (ROOT / "docs/stdlib/reference.md").read_text(encoding="utf-8")
    if "| Name | Signature | Module | Stability | Implementation |" not in reference:
        failures.append("central API reference lacks implementation axis")

    guarantee_ids = {item["id"] for item in release.get("guarantees", [])}
    for command in cli.get("commands", []):
        if "guarantee_tier" in command:
            failures.append(f"{command['id']} uses ambiguous guarantee_tier")
        unknown = set(command.get("guarantee_ids", [])) - guarantee_ids
        if unknown:
            failures.append(f"{command['id']} references unknown guarantees {sorted(unknown)}")
    for guarantee in release.get("guarantees", []):
        if guarantee.get("tier") == "guaranteed" and not guarantee.get("coverage"):
            failures.append(f"{guarantee['id']} lacks evidence scope")

    verification = project.get("verification", {})
    # Blockers are now owned by release-guarantees.toml, not project-state.toml
    release_checks = release.get("checks", [])
    failing_blockers = [ch for ch in release_checks if ch.get("release_blocking") and ch.get("result", ch.get("current_status")) == "fail"]
    fixture_count = sum(int(ch.get("affected_count", 1)) for ch in failing_blockers if ch.get("blocker_category") == "fixture")
    check_count = sum(int(ch.get("affected_count", 1)) for ch in failing_blockers if ch.get("blocker_category") == "verification")
    if fixture_count != verification.get("fixture_failures"):
        failures.append("fixture blocker rows do not account for published count")
    if check_count != verification.get("checks_total") - verification.get("checks_passed"):
        failures.append("verification blocker rows do not account for published count")

    spec = (ROOT / "docs/language/spec.md").read_text(encoding="utf-8")
    if "feature-arukellt-v1" in spec or re.search(r"Active \(v[013]\)", spec):
        failures.append("selectable-edition ambiguity remains in normative spec")

    current_paths = [ROOT / "docs/language/spec.md", ROOT / "docs/language/guide.md", ROOT / "docs/stdlib/cookbook.md"]
    skips = [match.group(0) for path in current_paths for match in re.finditer(r"<!--\s*skip-doc-check\b.*?-->", path.read_text(encoding="utf-8"))]
    expiries = {m.group(1) for item in skips if (m := re.search(r'expires="([^"]+)"', item))}
    owners = {m.group(1) for item in skips if (m := re.search(r'owner="([^"]+)"', item))}
    if len(skips) >= 125 or len(expiries) < 2 or len(owners) < 2:
        failures.append("high-volume suppression set was not reduced and staggered")

    for old_path in (
        "docs/compiler/t3-reachability.md",
        "docs/compiler/t3-rmw-optimization.md",
        "docs/process/linear-vs-gc-report.md",
        "docs/process/wasm-size-reduction.md",
    ):
        if (ROOT / old_path).exists():
            failures.append(f"legacy target-era document remains current: {old_path}")

    if failures:
        print("documentation publication contract: FAIL", file=sys.stderr)
        for failure in failures:
            print(f"  - {failure}", file=sys.stderr)
        return 1
    print("documentation publication contract: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
