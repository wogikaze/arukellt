#!/usr/bin/env python3
"""Validate docs/data/observable-semantics.toml against the fixture harness.

Issue #877 / RFC-011 item 45: the registry is the single specification
system for observable language rules. This checker keeps it honest:

- schema: every rule carries id/domain/statement/status/adrs; fixtures
  and diagnostics are path lists relative to tests/fixtures/
- coverage: each RFC-011 item-45 domain has at least one verified rule
- fixture honesty: every referenced path exists AND is registered in
  tests/fixtures/manifest.txt (any kind), so "verified" cannot cite
  fixtures the harness never runs
- ADR honesty: referenced ADRs exist and are ACCEPTED (PROPOSED or
  SUPERSEDED decisions cannot norm observable behaviour)
- tracking discipline: status != "verified" requires an issue reference

Usage: python3 scripts/check/check-observable-semantics.py
Exit: 0 when the registry is valid, 1 otherwise.
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:  # python < 3.11
    import tomli as tomllib

ROOT = Path(__file__).resolve().parent.parent.parent
REGISTRY = ROOT / "docs" / "data" / "observable-semantics.toml"
FIXTURE_MANIFEST = ROOT / "tests" / "fixtures" / "manifest.txt"
FIXTURES_ROOT = ROOT / "tests" / "fixtures"
ADR_DIR = ROOT / "docs" / "adr"

# RFC-011 item 45 names these seven observable domains. A domain without
# a verified rule means the spec system has a hole.
REQUIRED_DOMAINS = {
    "alias",
    "copy",
    "evaluation",
    "numeric",
    "unicode",
    "closure",
    "trait",
}
VALID_STATUSES = {"verified", "provisional", "unimplemented"}
ID_PATTERN = re.compile(r"^[a-z][a-z0-9_]*$")
ISSUE_PATTERN = re.compile(r"^#[0-9]+$")
ADR_PATTERN = re.compile(r"^ADR-[0-9]{3}$")


def load_manifest_paths() -> set[str]:
    paths: set[str] = set()
    for line in FIXTURE_MANIFEST.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#") or ":" not in line:
            continue
        paths.add(line.split(":", 1)[1].strip())
    return paths


def adr_is_accepted(adr_id: str) -> bool | None:
    """True when docs/adr/ADR-NNN-*.md exists and carries ACCEPTED status."""
    matches = sorted(ADR_DIR.glob(adr_id + "-*.md"))
    if not matches:
        return None
    head = "\n".join(matches[0].read_text(encoding="utf-8").splitlines()[:40])
    return "ACCEPTED" in head


def main() -> int:
    errors: list[str] = []
    if not REGISTRY.exists():
        print(f"error: registry not found: {REGISTRY}", file=sys.stderr)
        return 2
    data = tomllib.loads(REGISTRY.read_text(encoding="utf-8"))
    rules = data.get("rules", [])
    manifest_paths = load_manifest_paths()
    seen_ids: set[str] = set()
    verified_domains: set[str] = set()
    counts = {"verified": 0, "provisional": 0, "unimplemented": 0}

    for i, rule in enumerate(rules):
        loc = f"rules[{i}] ({rule.get('id', '?')})"
        rid = rule.get("id", "")
        if not ID_PATTERN.match(rid):
            errors.append(f"{loc}: id must be snake_case")
        elif rid in seen_ids:
            errors.append(f"{loc}: duplicate id")
        seen_ids.add(rid)

        domain = rule.get("domain", "")
        if domain not in REQUIRED_DOMAINS:
            errors.append(f"{loc}: unknown domain {domain!r}")

        if not str(rule.get("statement", "")).strip():
            errors.append(f"{loc}: statement must be non-empty")

        status = rule.get("status", "")
        if status not in VALID_STATUSES:
            errors.append(f"{loc}: status must be one of {sorted(VALID_STATUSES)}")
        else:
            counts[status] += 1
            if status == "verified" and domain in REQUIRED_DOMAINS:
                verified_domains.add(domain)

        issue = str(rule.get("issue", "")).strip()
        if status != "verified" and not ISSUE_PATTERN.match(issue):
            errors.append(f"{loc}: status={status} requires issue='#NNN'")

        adrs = rule.get("adrs", [])
        if not isinstance(adrs, list) or not adrs:
            errors.append(f"{loc}: adrs must be a non-empty list")
        else:
            for adr in adrs:
                if not ADR_PATTERN.match(str(adr)):
                    errors.append(f"{loc}: bad adr id {adr!r}")
                    continue
                accepted = adr_is_accepted(str(adr))
                if accepted is None:
                    errors.append(f"{loc}: {adr} not found in docs/adr/")
                elif not accepted:
                    errors.append(f"{loc}: {adr} is not ACCEPTED")

        for field in ("fixtures", "diagnostics"):
            paths = rule.get(field, [])
            if not isinstance(paths, list):
                errors.append(f"{loc}: {field} must be a list")
                continue
            if field == "fixtures" and not paths:
                errors.append(f"{loc}: fixtures must be non-empty")
            for rel in paths:
                rel = str(rel)
                if rel not in manifest_paths:
                    errors.append(f"{loc}: {rel} not in manifest.txt")
                elif not (FIXTURES_ROOT / rel).exists():
                    errors.append(f"{loc}: {rel} does not exist")

    for domain in sorted(REQUIRED_DOMAINS - verified_domains):
        errors.append(f"domain {domain!r}: no verified rule")

    for msg in errors:
        print(f"FAIL: {msg}")
    print(
        f"observable-semantics: {len(rules)} rules "
        f"(verified={counts['verified']}, provisional={counts['provisional']}, "
        f"unimplemented={counts['unimplemented']}), "
        f"{len(verified_domains)}/{len(REQUIRED_DOMAINS)} domains verified"
    )
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
