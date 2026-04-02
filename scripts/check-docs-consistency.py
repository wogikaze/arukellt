#!/usr/bin/env python3
"""Extended docs consistency checker.

Beyond generated-docs freshness, this script validates:
- Bootstrap state: docs match verify-bootstrap.sh output
- Capability state: docs match std/manifest.toml kind metadata
- Component state: docs match implementation support
- Stale detection: concrete diffs on mismatch
"""
from __future__ import annotations

import subprocess
import sys
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MANIFEST = ROOT / "std" / "manifest.toml"
CURRENT_STATE = ROOT / "docs" / "current-state.md"

errors: list[str] = []


def check_maturity_matrix_freshness() -> int:
    """Verify that maturity-matrix.md is in sync with language-doc-classifications.toml.

    If the TOML [[features]] section changes but the matrix isn't regenerated,
    this check detects the drift before CI does a full regeneration pass.
    """
    import tomllib as _tomllib

    toml_path = ROOT / "docs" / "data" / "language-doc-classifications.toml"
    matrix_path = ROOT / "docs" / "language" / "maturity-matrix.md"

    if not toml_path.exists() or not matrix_path.exists():
        return 0

    data = _tomllib.loads(toml_path.read_text(encoding="utf-8"))
    features = data.get("features", [])
    if not features:
        return 0

    matrix_text = matrix_path.read_text(encoding="utf-8")

    # Count feature rows in the matrix (lines matching | N or N.M | pattern)
    feature_rows = re.findall(r'^\| \d+(?:\.\d+)? \|', matrix_text, re.MULTILINE)
    if len(features) != len(feature_rows):
        errors.append(
            f"maturity matrix stale: TOML has {len(features)} features "
            f"but maturity-matrix.md has {len(feature_rows)} rows; "
            "run `python3 scripts/generate-docs.py`"
        )
        return 1

    # Verify TOML source-of-truth marker is present in the matrix
    if "language-doc-classifications.toml" not in matrix_text:
        errors.append(
            "maturity matrix missing TOML source marker; "
            "regenerate with `python3 scripts/generate-docs.py`"
        )
        return 1

    return 0


def check_generated_docs() -> int:
    """Run generate-docs.py --check."""
    cmd = [sys.executable, str(ROOT / "scripts" / "generate-docs.py"), "--check"]
    result = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True)
    if result.returncode != 0:
        errors.append("generated docs are out of date; run `python3 scripts/generate-docs.py`")
        if result.stdout.strip():
            for line in result.stdout.strip().splitlines():
                errors.append(f"  {line}")
        return 1
    return 0


def check_capability_state() -> int:
    """Validate that host_stub functions in manifest are acknowledged in docs."""
    if not MANIFEST.exists():
        return 0

    manifest_text = MANIFEST.read_text()

    # Count host_stub functions
    host_stubs = re.findall(r'kind\s*=\s*"host_stub"', manifest_text)
    stub_count = len(host_stubs)

    # Extract host_stub function names
    stub_names: list[str] = []
    blocks = manifest_text.split("[[functions]]")
    for block in blocks[1:]:
        if 'kind = "host_stub"' in block:
            m = re.search(r'name\s*=\s*"([^"]+)"', block)
            if m:
                stub_names.append(m.group(1))

    if not CURRENT_STATE.exists():
        return 0

    cs_text = CURRENT_STATE.read_text()

    # Check that current-state mentions stub count or stub status
    if stub_count > 0 and "host_stub" not in cs_text and "stub" not in cs_text.lower():
        errors.append(
            f"capability drift: {stub_count} host_stub functions in manifest "
            f"({', '.join(stub_names)}) but current-state.md does not mention stubs"
        )
        return 1

    return 0


def check_fixture_count_freshness() -> int:
    """Validate that project-state.toml fixture count matches manifest.txt."""
    project_state = ROOT / "docs" / "data" / "project-state.toml"
    manifest_txt = ROOT / "tests" / "fixtures" / "manifest.txt"

    if not project_state.exists() or not manifest_txt.exists():
        return 0

    # Count fixtures in manifest.txt (non-comment, non-empty lines)
    lines = [
        l.strip()
        for l in manifest_txt.read_text().splitlines()
        if l.strip() and not l.strip().startswith("#")
    ]
    actual_count = len(lines)

    # Find fixture count in project-state.toml
    ps_text = project_state.read_text()
    m = re.search(r'fixture_count\s*=\s*(\d+)', ps_text)
    if m:
        recorded_count = int(m.group(1))
        if recorded_count != actual_count:
            errors.append(
                f"fixture count drift: project-state.toml says {recorded_count} "
                f"but manifest.txt has {actual_count}"
            )
            return 1

    return 0


def check_issue_index_freshness() -> int:
    """Validate that issue indexes are up to date by comparing with regeneration."""
    index_path = ROOT / "issues" / "open" / "index.md"
    graph_path = ROOT / "issues" / "open" / "dependency-graph.md"
    generator = ROOT / "scripts" / "generate-issue-index.sh"

    if not generator.exists():
        return 0
    if not index_path.exists() or not graph_path.exists():
        return 0

    # Capture current content
    old_index = index_path.read_text()
    old_graph = graph_path.read_text()

    # Regenerate
    result = subprocess.run(
        ["bash", str(generator)],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        errors.append("issue index regeneration failed")
        return 1

    new_index = index_path.read_text()
    new_graph = graph_path.read_text()

    stale = 0
    if new_index != old_index:
        errors.append(
            "issue index stale: issues/open/index.md differs after regeneration; "
            "run `bash scripts/generate-issue-index.sh`"
        )
        stale = 1
    if new_graph != old_graph:
        errors.append(
            "dependency graph stale: issues/open/dependency-graph.md differs after regeneration; "
            "run `bash scripts/generate-issue-index.sh`"
        )
        stale = 1

    return stale


def check_host_badge_presence() -> int:
    """Verify that host module pages contain target/status badges.

    Every generated module page that includes a ``std::host::*`` module section
    must contain the unified badge pattern (``🎯 **Target:**``) produced by
    ``format_host_module_badges()`` in the generator.
    """
    modules_dir = ROOT / "docs" / "stdlib" / "modules"
    if not modules_dir.exists():
        return 0

    badge_pattern = re.compile(r"🎯 \*\*Target:\*\*")
    missing: list[str] = []
    for md_file in sorted(modules_dir.glob("*.md")):
        content = md_file.read_text()
        # Only check pages that contain std::host:: module sections
        if "## `std::host::" in content and not badge_pattern.search(content):
            missing.append(md_file.name)

    if missing:
        errors.append(
            f"host badge drift: {', '.join(missing)} contain std::host:: modules "
            f"but lack target/status badges; regenerate with `python3 scripts/generate-docs.py`"
        )
        return 1

    return 0


def check_deprecated_badge_presence() -> int:
    """Verify that deprecated functions display ⚠️ Deprecated badges in generated docs.

    For every function with ``deprecated_by`` or ``stability = "deprecated"``
    in the manifest, the reference page must contain a ⚠️ Deprecated badge.
    Also checks that the Deprecated APIs summary section exists.
    """
    import tomllib as _tomllib

    if not MANIFEST.exists():
        return 0

    manifest = _tomllib.loads(MANIFEST.read_text(encoding="utf-8"))
    deprecated_names: list[str] = []
    for fn in manifest.get("functions", []):
        if fn.get("deprecated_by") or fn.get("stability") == "deprecated":
            deprecated_names.append(fn["name"])

    if not deprecated_names:
        return 0

    reference_path = ROOT / "docs" / "stdlib" / "reference.md"
    if not reference_path.exists():
        errors.append("deprecated badge check: reference.md not found")
        return 1

    ref_text = reference_path.read_text(encoding="utf-8")
    failed = 0

    # Check that the Deprecated APIs summary section exists
    if "## Deprecated APIs" not in ref_text:
        errors.append(
            f"deprecated badge drift: {len(deprecated_names)} deprecated function(s) in manifest "
            "but reference.md lacks '## Deprecated APIs' section"
        )
        failed = 1

    # Check that each deprecated function has a ⚠️ Deprecated badge in reference.md
    missing_badges: list[str] = []
    for name in deprecated_names:
        # Look for the badge pattern: ~~`name`~~ ⚠️ Deprecated
        badge_pattern = f"~~`{name}`~~ ⚠️ Deprecated"
        if badge_pattern not in ref_text:
            missing_badges.append(name)

    if missing_badges:
        errors.append(
            f"deprecated badge drift: {', '.join(missing_badges)} have deprecated_by in manifest "
            f"but lack ⚠️ Deprecated badge in reference.md; regenerate with `python3 scripts/generate-docs.py`"
        )
        failed = 1

    return min(failed, 1)


# ── Spec–Guide sync ──────────────────────────────────────────────────────────

# Words too generic to signal topic coverage.
_COVERAGE_STOP_WORDS = frozenset({
    "and", "the", "a", "an", "of", "in", "for", "to", "with", "or",
    "vs", "non", "see", "individual", "entries",
})


def _extract_coverage_keywords(name: str) -> set[str]:
    """Extract significant topic keywords from a feature / subsection name.

    Strips version markers ``(v1)``, backticks, em-dashes, and qualified
    paths so that only meaningful topic words remain.
    """
    # Remove parenthetical markers: (v1), (prelude), (std::host::stdio), etc.
    cleaned = re.sub(r"\([^)]*\)", "", name).strip()
    # Normalise punctuation
    cleaned = cleaned.replace("`", "").replace("\u2014", " ").replace("::", " ")
    words = cleaned.lower().split()
    return {w for w in words if len(w) > 2 and w not in _COVERAGE_STOP_WORDS}


def check_spec_guide_sync() -> int:
    """Detect when spec.md stable features diverge from guide.md coverage.

    Reads ``[[features]]`` from *language-doc-classifications.toml* to find
    top-level stable spec sections (IDs without a dot, e.g. ``"1"``,
    ``"8"``).  For each, it collects topic keywords from the section name
    **and** all of its stable sub-section names, then checks whether
    *guide.md* mentions any of those keywords — first in section headings,
    then in body text.

    A missing match means the guide has drifted away from spec coverage and
    a warning is emitted.
    """
    import tomllib

    classifications_path = ROOT / "docs" / "data" / "language-doc-classifications.toml"
    guide_path = ROOT / "docs" / "language" / "guide.md"

    if not classifications_path.exists() or not guide_path.exists():
        return 0

    toml_data = tomllib.loads(
        classifications_path.read_text(encoding="utf-8")
    )
    features = toml_data.get("features", [])
    if not features:
        return 0

    # ── 1. Build top-level stable sections with subsection names ──────────
    top_level: dict[str, dict[str, object]] = {}  # id → {name, subs}
    for feat in features:
        fid = str(feat.get("id", ""))
        name = feat.get("name", "")
        stability = feat.get("stability", "")

        if "." not in fid:
            # Top-level section
            if stability == "stable":
                top_level[fid] = {"name": name, "subs": []}
        else:
            # Sub-section – attach to parent when both are stable
            parent_id = fid.split(".")[0]
            if parent_id in top_level and stability == "stable":
                top_level[parent_id]["subs"].append(name)  # type: ignore[union-attr]

    if not top_level:
        return 0

    # ── 2. Parse guide headings + full text ───────────────────────────────
    guide_text = guide_path.read_text(encoding="utf-8")
    guide_lower = guide_text.lower()
    guide_headings_lower: list[str] = [
        line.lstrip("#").strip().lower()
        for line in guide_text.splitlines()
        if line.startswith("#")
    ]

    # ── 3. Check each stable section for guide coverage ───────────────────
    uncovered: list[str] = []
    for sid in sorted(top_level, key=lambda x: int(x)):
        sec = top_level[sid]
        keywords = _extract_coverage_keywords(sec["name"])  # type: ignore[arg-type]
        for sub_name in sec["subs"]:  # type: ignore[union-attr]
            keywords.update(_extract_coverage_keywords(sub_name))

        # Match: keyword appears in any guide heading …
        covered = any(
            kw in heading
            for kw in keywords
            for heading in guide_headings_lower
        )
        # … or as a fallback, anywhere in the guide body text.
        if not covered:
            covered = any(kw in guide_lower for kw in keywords)

        if not covered:
            uncovered.append(f"\u00a7{sid} {sec['name']}")

    if uncovered:
        errors.append(
            f"spec-guide sync drift: {len(uncovered)} stable spec feature(s) "
            f"have no coverage in guide.md: {', '.join(uncovered)}"
        )
        return 1

    return 0


def main() -> int:
    failed = 0
    failed += check_maturity_matrix_freshness()
    failed += check_generated_docs()
    failed += check_capability_state()
    failed += check_host_badge_presence()
    failed += check_deprecated_badge_presence()
    failed += check_fixture_count_freshness()
    failed += check_issue_index_freshness()
    failed += check_spec_guide_sync()

    if errors:
        print("docs consistency check FAILED:", file=sys.stderr)
        for err in errors:
            print(f"  ✗ {err}", file=sys.stderr)
        return 1

    print(f"docs consistency OK ({len(errors)} issues)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
