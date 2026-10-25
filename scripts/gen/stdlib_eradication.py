"""ADR-046 stdlib free-function eradication classification.

Shared by scripts/gen/generate-docs.py (scorecard + inventory doc) and
scripts/check/check-stdlib-eradication.py (coverage enforcement).

Two surfaces are classified:

  * ``std/manifest.toml`` ``[[functions]]`` entries — the registered
    public API (the historical scope of the scorecard).
  * top-level ``pub fn`` declarations under ``std/**/*.ark`` whose name
    has no manifest entry — the unregistered public free functions that
    made the "0 public free functions" goal unmeasurable before #703.

The rule list lives in ``docs/data/stdlib-api-eradication.toml``; rules
are evaluated in order and the first match wins. ``[source_default]`` is
the catch-all for unregistered declarations: matching it is an error
(``unregistered``), not a classification.
"""
from __future__ import annotations

import re
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:  # Python < 3.11
    import tomli as tomllib

CLASSES = ("must delete", "associated-or-method", "intrinsic-only")
PUBLIC_CLASSES = ("must delete", "associated-or-method")

# Pseudo-class used only by ``[source_default]``. A declaration that falls
# through every explicit source rule is an unregistered public free
# function, which the gate reports instead of silently classifying.
SOURCE_ERROR_CLASS = "unregistered"

# Only column-0 declarations are module-level free functions. Indented
# ``pub fn`` lines are methods inside an ``impl`` block and are not part
# of the free-function surface.
SOURCE_DECL_RE = re.compile(r"^pub fn ([A-Za-z_][A-Za-z0-9_]*)", re.MULTILINE)


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
    if rule.get("source_only") and not entry.get("source"):
        return False
    if rule.get("manifest_only") and entry.get("source"):
        return False
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
    paths = _as_list(rule.get("path"))
    if paths and entry.get("path") not in paths:
        return False
    path_pfxs = _as_list(rule.get("path_pfx"))
    if path_pfxs and not any(entry.get("path", "").startswith(p) for p in path_pfxs):
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


def resolve_source_class(eradication: dict, entry: dict) -> dict | None:
    """Return the first matching rule, falling back to the source catch-all."""
    rule = resolve_class(eradication, entry)
    if rule is not None:
        return rule
    return eradication.get("source_default")


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


def module_for_path(root: Path, path: Path) -> str | None:
    """Map a ``std/**/*.ark`` file to its module path.

    ``std/text/mod.ark`` -> ``std::text``; ``std/prelude.ark`` -> ``None``
    (prelude scope); other files use the path stem.
    """
    rel = path.resolve().relative_to((root / "std").resolve()).with_suffix("")
    if rel.as_posix() == "prelude":
        return None
    parts = list(rel.parts)
    if parts and parts[-1] == "mod":
        parts = parts[:-1]
    if not parts:
        return None
    return "std::" + "::".join(parts)


def iter_source_declarations(root: Path) -> list[dict]:
    """Enumerate top-level ``pub fn`` declarations under ``std/**/*.ark``."""
    declarations = []
    std_root = root / "std"
    for path in sorted(std_root.rglob("*.ark")):
        text = path.read_text(encoding="utf-8")
        module = module_for_path(root, path)
        for match in SOURCE_DECL_RE.finditer(text):
            entry = {
                "name": match.group(1),
                "path": path.relative_to(root).as_posix(),
                "line": text.count("\n", 0, match.start()) + 1,
                "source": True,
            }
            if module is not None:
                entry["module"] = module
            declarations.append(entry)
    return declarations


def classify_source(eradication: dict, manifest: dict, declarations: list[dict]):
    """Classify unregistered top-level std declarations.

    Declarations whose name is already represented by a manifest entry are
    skipped there: their class comes from ``classify_manifest`` and adding
    them again would double-count the scorecard denominator.

    Returns (resolved, unregistered, errors). ``unregistered`` is the full
    source-only list (the scorecard numerator), including entries that
    resolved to the catch-all and produced an error.
    """
    registered = {entry.get("name") for entry in manifest.get("functions", [])}
    resolved = []
    unregistered = []
    errors = []
    for entry in declarations:
        if entry["name"] in registered:
            continue
        unregistered.append(entry)
        rule = resolve_source_class(eradication, entry)
        if rule is None:
            errors.append(
                "unregistered std free: {}:{} {} — add a source rule or register "
                "it in std/manifest.toml".format(
                    entry["path"], entry["line"], entry["name"]))
            continue
        cls = rule.get("class", "")
        if cls == SOURCE_ERROR_CLASS:
            errors.append(
                "unregistered std free: {}:{} {} — catches source_default; add an "
                "explicit source rule or register it in std/manifest.toml".format(
                    entry["path"], entry["line"], entry["name"]))
            continue
        if cls not in CLASSES:
            errors.append("{}: rule {!r} has invalid class {!r}".format(
                entry["name"], rule_label(rule), cls))
            continue
        resolved.append((entry, rule))
    return resolved, unregistered, errors


def entry_requires_decl(entry: dict) -> bool:
    """Whether a manifest entry claims a declaration in std source.

    Compiler-provided entries (``builtin`` / ``intrinsic`` / ``host_stub``)
    are implemented outside ``std/**/*.ark``. Prelude wrappers claim a
    ``std/prelude.ark`` function. Module-scoped entries without a ``kind``
    are the "module" role and claim a declaration in their module file.
    """
    name = entry.get("name", "")
    if name.startswith("__intrinsic_"):
        return False
    kind = entry.get("kind")
    if kind in ("builtin", "intrinsic", "host_stub"):
        return False
    if kind == "prelude_wrapper":
        return True
    return bool(entry.get("module"))


def manifest_entries_without_decl(manifest: dict, decl_names: set[str]) -> list[dict]:
    """Manifest entries that claim a std declaration but have none."""
    return [
        entry
        for entry in manifest.get("functions", [])
        if entry_requires_decl(entry) and entry.get("name") not in decl_names
    ]


def intrinsic_is_non_public(entry: dict) -> bool:
    """True only for the ADR-046 intrinsic-only form.

    ``kind = "intrinsic_wrapper"`` is a public wrapper kind, so it must not
    be accepted as an intrinsic-only escape hatch.
    """
    return (
        entry.get("name", "").startswith("__intrinsic_")
        or entry.get("kind") == "intrinsic"
    )


def owner_issue_path(root: Path, owner: str):
    for bucket in ("open", "done", "reject"):
        for path in (root / "issues" / bucket).glob("{}-*.md".format(owner)):
            return path
    return None
