"""ADR-046 stdlib free-function eradication classification.

Shared by scripts/gen/generate-docs.py (scorecard + inventory doc) and
scripts/check/check-stdlib-eradication.py (coverage enforcement).
The rule list lives in docs/data/stdlib-api-eradication.toml; rules are
evaluated in order and the first match wins.
"""
from __future__ import annotations

from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:  # Python < 3.11
    import tomli as tomllib

CLASSES = ("must delete", "associated-or-method", "intrinsic-only")
PUBLIC_CLASSES = ("must delete", "associated-or-method")


def rule_label(rule: dict) -> str:
    return str(rule.get("label") or rule.get("name") or "(unnamed rule)")


def load_eradication(path: Path) -> dict:
    with open(path, "rb") as handle:
        return tomllib.load(handle)


def _as_list(value) -> list:
    if value is None:
        return []
    if isinstance(value, list):
        return value
    return [value]


def entry_is_deprecated(entry: dict) -> bool:
    return bool(entry.get("deprecated_by")) or entry.get("stability") == "deprecated"


def rule_matches(rule: dict, entry: dict) -> bool:
    name = entry.get("name", "")
    names = _as_list(rule.get("name"))
    if names and name not in names:
        return False
    prefixes = _as_list(rule.get("prefix"))
    if prefixes and not any(name.startswith(p) for p in prefixes):
        return False
    kinds = _as_list(rule.get("kind"))
    if kinds and entry.get("kind") not in kinds:
        return False
    modules = _as_list(rule.get("module"))
    if modules and entry.get("module") not in modules:
        return False
    module_pfxs = _as_list(rule.get("module_pfx"))
    if module_pfxs and not any(entry.get("module", "").startswith(m) for m in module_pfxs):
        return False
    if "module_less" in rule:
        if bool(rule["module_less"]) != ("module" not in entry):
            return False
    if "deprecated" in rule:
        if bool(rule["deprecated"]) != entry_is_deprecated(entry):
            return False
    return True


def resolve_class(eradication: dict, entry: dict) -> dict | None:
    """Return the first matching rule for a manifest function entry."""
    for rule in eradication.get("rules", []):
        if rule_matches(rule, entry):
            return rule
    return None


def classify_manifest(eradication: dict, manifest: dict):
    """Resolve every manifest function to (entry, rule).

    Returns (resolved, unclassified, errors). errors covers invalid class
    vocabulary.
    """
    resolved = []
    unclassified = []
    errors = []
    for entry in manifest.get("functions", []):
        rule = resolve_class(eradication, entry)
        if rule is None:
            unclassified.append(entry)
            continue
        cls = rule.get("class", "")
        if cls not in CLASSES:
            errors.append("{}: rule {!r} has invalid class {!r}".format(
                entry.get("name"), rule_label(rule), cls))
            continue
        resolved.append((entry, rule))
    return resolved, unclassified, errors


def owner_issue_path(root: Path, owner: str):
    for bucket in ("open", "done", "reject"):
        for path in (root / "issues" / bucket).glob("{}-*.md".format(owner)):
            return path
    return None
