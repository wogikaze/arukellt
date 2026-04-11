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

try:
    import tomllib as _tomllib
except ModuleNotFoundError:  # pragma: no cover - Python < 3.11 local compatibility
    import tomli as _tomllib

ROOT = Path(__file__).resolve().parent.parent.parent
MANIFEST = ROOT / "std" / "manifest.toml"
CURRENT_STATE = ROOT / "docs" / "current-state.md"

errors: list[str] = []


def check_maturity_matrix_freshness() -> int:
    """Verify that maturity-matrix.md is in sync with language-doc-classifications.toml.

    If the TOML [[features]] section changes but the matrix isn't regenerated,
    this check detects the drift before CI does a full regeneration pass.
    """
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
    cmd = [sys.executable, str(ROOT / "scripts" / "gen" / "generate-docs.py"), "--check"]
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
    classifications_path = ROOT / "docs" / "data" / "language-doc-classifications.toml"
    guide_path = ROOT / "docs" / "language" / "guide.md"

    if not classifications_path.exists() or not guide_path.exists():
        return 0

    toml_data = _tomllib.loads(
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


# ── Feature-level spec–guide drift (#415) ─────────────────────────────────────

# Top-level spec sections whose subsections are foundational syntax descriptions
# (e.g. source encoding, whitespace, identifiers) covered implicitly by every
# code example in the guide.  Subsections under these parents are excluded from
# the feature-level drift check.
_FOUNDATIONAL_PARENT_SECTIONS = frozenset({"1"})

# Individual subsections that are syntax grammar rules rather than user-facing
# features.  These describe notation/grammar mechanics and are covered
# implicitly through guide examples.
_FOUNDATIONAL_SUBSECTIONS = frozenset({
    "3.2",  # Identifier and Qualified Identifier — syntax grammar
})

# Maximum individually-uncovered stable subsections allowed before the check
# fails.  Accommodates minor transient gaps when new spec subsections are added
# but the guide hasn't been updated yet.
_FEATURE_DRIFT_TOLERANCE = 2

# Extended stop-word list for body-content keyword extraction, filtering out
# prose words that provide no topical signal.
_CONTENT_STOP_WORDS = _COVERAGE_STOP_WORDS | frozenset({
    "that", "this", "from", "when", "each", "both", "same",
    "must", "have", "been", "will", "they", "them", "than",
    "type", "used", "uses", "into", "also", "only", "which",
    "such", "like", "then", "some", "more", "does", "note",
    "example", "above", "below", "following", "per", "are",
    "not", "can", "all", "its", "one", "two", "any",
})


def _parse_spec_subsection_bodies(spec_text: str) -> dict[str, str]:
    """Parse spec.md and return subsection body text keyed by section ID.

    Returns a mapping from dotted IDs (e.g. ``"3.2"``, ``"9.12"``) to the
    body text between that heading and the next ``###`` heading.
    """
    heading_re = re.compile(r"^###\s+(\d+\.\d+)\s+", re.MULTILINE)
    matches = list(heading_re.finditer(spec_text))
    bodies: dict[str, str] = {}
    for i, m in enumerate(matches):
        sid = m.group(1)
        start = m.end()
        end = matches[i + 1].start() if i + 1 < len(matches) else len(spec_text)
        bodies[sid] = spec_text[start:end]
    return bodies


def _extract_content_keywords(body: str) -> set[str]:
    """Extract significant topic words from a spec subsection body.

    Strips code blocks, inline code, and table formatting, then returns
    words that are long enough and topically meaningful.  Uses a stricter
    filter than ``_extract_coverage_keywords()`` to reduce prose noise.
    """
    text = re.sub(r"```.*?```", "", body, flags=re.DOTALL)
    text = re.sub(r"`[^`]*`", "", text)
    text = re.sub(r"[|#\-]", " ", text)
    text = re.sub(r"[^\w\s']", " ", text)
    words = text.lower().split()
    return {
        w
        for w in words
        if len(w) > 3
        and w not in _CONTENT_STOP_WORDS
        and not w.isdigit()
        and not w.startswith("__")
    }


def check_spec_guide_feature_drift() -> int:
    """Detect stable spec subsections missing from guide.md at feature level.

    Extends ``check_spec_guide_sync()`` with subsection-level granularity.
    For each stable subsection in *language-doc-classifications.toml*,
    extracts topic keywords from both the feature name **and** spec.md body
    content, then checks whether *guide.md* mentions any of those keywords.

    Subsections under foundational parent sections (§1 Lexical Structure)
    and individual foundational subsections (§3.2 Identifiers) are excluded.

    Reports uncovered features grouped by parent section.  Tolerates up to
    ``_FEATURE_DRIFT_TOLERANCE`` uncovered features before failing, to
    accommodate transient gaps when new spec content is added.
    """
    classifications_path = ROOT / "docs" / "data" / "language-doc-classifications.toml"
    guide_path = ROOT / "docs" / "language" / "guide.md"
    spec_path = ROOT / "docs" / "language" / "spec.md"

    if not all(p.exists() for p in [classifications_path, guide_path, spec_path]):
        return 0

    toml_data = _tomllib.loads(
        classifications_path.read_text(encoding="utf-8")
    )
    features = toml_data.get("features", [])
    if not features:
        return 0

    # ── 1. Parse spec.md for subsection body content ──────────────────────
    spec_text = spec_path.read_text(encoding="utf-8")
    spec_bodies = _parse_spec_subsection_bodies(spec_text)

    # Build parent-section name lookup
    parent_names: dict[str, str] = {}
    for feat in features:
        fid = str(feat.get("id", ""))
        if "." not in fid:
            parent_names[fid] = feat.get("name", "")

    # ── 2. Collect stable subsections, excluding foundational ones ────────
    stable_subs: list[tuple[str, str]] = []  # (id, name)
    for feat in features:
        fid = str(feat.get("id", ""))
        if "." not in fid:
            continue
        if feat.get("stability") != "stable":
            continue
        parent_id = fid.split(".")[0]
        if parent_id in _FOUNDATIONAL_PARENT_SECTIONS:
            continue
        if fid in _FOUNDATIONAL_SUBSECTIONS:
            continue
        stable_subs.append((fid, feat.get("name", "")))

    if not stable_subs:
        return 0

    # ── 3. Parse guide headings + full text ───────────────────────────────
    guide_text = guide_path.read_text(encoding="utf-8")
    guide_lower = guide_text.lower()
    guide_headings_lower: list[str] = [
        line.lstrip("#").strip().lower()
        for line in guide_text.splitlines()
        if line.startswith("#")
    ]

    # ── 4. Check each subsection for guide coverage ───────────────────────
    uncovered: list[tuple[str, str]] = []
    for fid, name in stable_subs:
        # Keywords from the feature title
        keywords = _extract_coverage_keywords(name)

        # Supplement with keywords from spec.md body content
        body = spec_bodies.get(fid, "")
        if body:
            keywords |= _extract_content_keywords(body)

        if not keywords:
            continue

        # Match: any keyword in a guide heading …
        covered = any(
            kw in heading
            for kw in keywords
            for heading in guide_headings_lower
        )
        # … or anywhere in the guide body text.
        if not covered:
            covered = any(kw in guide_lower for kw in keywords)

        if not covered:
            uncovered.append((fid, name))

    if not uncovered:
        return 0

    # ── 5. Group by parent and report ─────────────────────────────────────
    by_parent: dict[str, list[str]] = {}
    for fid, name in uncovered:
        parent_id = fid.split(".")[0]
        parent_name = parent_names.get(parent_id, "")
        key = f"\u00a7{parent_id} {parent_name}"
        by_parent.setdefault(key, []).append(f"\u00a7{fid} {name}")

    detail_lines = []
    for parent_key in sorted(by_parent, key=lambda k: int(k.split()[0].lstrip("\u00a7"))):
        items = by_parent[parent_key]
        detail_lines.append(f"  {parent_key}: {', '.join(items)}")

    # Below tolerance → informational note, not a failure
    if len(uncovered) <= _FEATURE_DRIFT_TOLERANCE:
        print(
            f"spec-guide feature drift (info): {len(uncovered)} stable subsection(s) "
            "lack guide.md coverage (within tolerance):",
            file=sys.stderr,
        )
        for line in detail_lines:
            print(f"  \u2139 {line}", file=sys.stderr)
        return 0

    # Above tolerance → error
    errors.append(
        f"spec-guide feature drift: {len(uncovered)} stable subsection(s) "
        f"lack guide.md coverage (tolerance: {_FEATURE_DRIFT_TOLERANCE}):\n"
        + "\n".join(detail_lines)
    )
    return 1


# ── Metadata-level verification (#403) ────────────────────────────────────────


def _parse_manifest() -> dict:
    """Load and return parsed manifest.toml."""
    return _tomllib.loads(MANIFEST.read_text(encoding="utf-8"))


def _find_reference_row(
    ref_lines: list[str], fn_name: str, module: str | None = None
) -> str | None:
    """Find the reference.md table row containing a function by name and module.

    Returns the full line or None.  Handles both normal and deprecated display
    forms.  When *module* is given, the Module column (3rd pipe-field) must
    contain it to disambiguate functions that share a name (e.g. prelude
    ``get`` vs ``std::host::http`` ``get``).
    """
    normal_pattern = f"| `{fn_name}` |"
    deprecated_pattern = f"| ~~`{fn_name}`~~"
    for line in ref_lines:
        if normal_pattern in line or deprecated_pattern in line:
            if module is not None:
                cells = [c.strip() for c in line.split("|")]
                # cells: ['', Name, Signature, Module, Stability, Kind, ...]
                if len(cells) >= 4:
                    ref_module = cells[3].strip("`").strip()
                    if ref_module != module:
                        continue
            return line
    return None


def check_target_metadata_in_reference() -> int:
    """Verify that every function with ``target`` in manifest shows target info in reference.md.

    For each function entry carrying a ``target`` list (e.g. ``["wasm32-wasi-p2"]``),
    the corresponding reference.md row's Kind column must contain the target value
    in parentheses — e.g. ``(wasm32-wasi-p2)``.
    """
    if not MANIFEST.exists():
        return 0

    reference_path = ROOT / "docs" / "stdlib" / "reference.md"
    if not reference_path.exists():
        return 0

    manifest = _parse_manifest()
    ref_lines = reference_path.read_text(encoding="utf-8").splitlines()

    missing: list[str] = []
    for fn in manifest.get("functions", []):
        target_list = fn.get("target")
        if not target_list:
            continue
        # Skip raw intrinsics — they are not rendered into reference.md
        if fn.get("kind") == "intrinsic":
            continue
        name = fn["name"]
        module = fn.get("module", "prelude")
        row = _find_reference_row(ref_lines, name, module)
        if row is None:
            missing.append(f"{name} (module={module}, not found in reference.md)")
            continue
        # Each target value should appear in the row
        for target in target_list:
            if target not in row:
                missing.append(f"{name}: target '{target}' missing from Kind column")

    if missing:
        errors.append(
            f"target metadata drift in reference.md: "
            + "; ".join(missing)
            + "; regenerate with `python3 scripts/generate-docs.py`"
        )
        return 1

    return 0


def check_stability_metadata_in_reference() -> int:
    """Verify that every function's stability in manifest matches reference.md.

    Parses the Stability column from each function's table row and compares it
    to the manifest ``stability`` field.  Reports mismatches with the expected
    vs actual value.
    """
    if not MANIFEST.exists():
        return 0

    reference_path = ROOT / "docs" / "stdlib" / "reference.md"
    if not reference_path.exists():
        return 0

    manifest = _parse_manifest()
    ref_lines = reference_path.read_text(encoding="utf-8").splitlines()

    mismatches: list[str] = []
    for fn in manifest.get("functions", []):
        name = fn["name"]
        expected_stability = fn.get("stability")
        if not expected_stability:
            continue
        # Skip raw intrinsics — they are not rendered into reference.md
        if fn.get("kind") == "intrinsic":
            continue
        module = fn.get("module", "prelude")
        row = _find_reference_row(ref_lines, name, module)
        if row is None:
            mismatches.append(f"{name} (module={module}, not found in reference.md)")
            continue

        # Parse stability from table row — it's the 4th pipe-delimited column
        # Format: | Name | Signature | Module | Stability | Kind | Prelude | Intrinsic |
        cells = [c.strip() for c in row.split("|")]
        # cells[0] is empty (before first |), cells[1]=Name, ..., cells[4]=Stability
        if len(cells) >= 5:
            ref_stability = cells[4].strip("`").strip()
            if ref_stability != expected_stability:
                mismatches.append(
                    f"{name}: manifest='{expected_stability}', "
                    f"reference.md='{ref_stability}'"
                )

    if mismatches:
        errors.append(
            f"stability metadata drift in reference.md: "
            + "; ".join(mismatches)
            + "; regenerate with `python3 scripts/generate-docs.py`"
        )
        return 1

    return 0


def check_cross_page_metadata_consistency() -> int:
    """Verify that reference.md and module pages agree on metadata display.

    For each module page under ``docs/stdlib/modules/``, extracts function
    names and their stability from the generated tables, then cross-checks
    against the manifest source of truth.  Also verifies that host module
    pages display ``🎯 **Target:**`` badges consistent with the manifest
    ``[[modules]]`` target metadata.
    """
    if not MANIFEST.exists():
        return 0

    modules_dir = ROOT / "docs" / "stdlib" / "modules"
    if not modules_dir.exists():
        return 0

    manifest = _parse_manifest()

    # Build lookup: function name → manifest metadata
    fn_lookup: dict[str, dict] = {}
    for fn in manifest.get("functions", []):
        fn_lookup[fn["name"]] = fn

    # Build lookup: module name → manifest module metadata
    mod_lookup: dict[str, dict] = {}
    for mod in manifest.get("modules", []):
        mod_lookup[mod["name"]] = mod

    inconsistencies: list[str] = []

    # Pattern for function names in module page tables:
    #   | `name` | ... | `stability` | ...
    fn_row_pattern = re.compile(
        r"^\| `([^`]+)` \|"  # function name in first column
    )
    # Pattern for stability column in module page tables (3rd pipe-column):
    #   | Name | Signature | Stability | ...
    mod_table_pattern = re.compile(
        r"^\| `([^`]+)` \| [^|]+ \| `([^`]+)` \|"
    )

    badge_pattern = re.compile(r"🎯 \*\*Target:\*\* `([^`]+)`")

    for md_file in sorted(modules_dir.glob("*.md")):
        content = md_file.read_text(encoding="utf-8")
        page_name = md_file.name

        # ── 1. Cross-check function stability in module pages vs manifest ──
        for match in mod_table_pattern.finditer(content):
            fn_name = match.group(1)
            page_stability = match.group(2)
            manifest_fn = fn_lookup.get(fn_name)
            if manifest_fn is None:
                continue
            expected_stability = manifest_fn.get("stability", "")
            if page_stability != expected_stability:
                inconsistencies.append(
                    f"{page_name}/{fn_name}: stability "
                    f"page='{page_stability}', manifest='{expected_stability}'"
                )

        # ── 2. Cross-check host module target badges vs manifest modules ──
        # Find all module headings (## `std::host::*`) and their badge lines
        module_heading_pattern = re.compile(
            r'^## `(std::host::[^`]+)`', re.MULTILINE
        )
        for heading_match in module_heading_pattern.finditer(content):
            mod_name = heading_match.group(1)
            mod_meta = mod_lookup.get(mod_name)
            if mod_meta is None:
                continue

            manifest_targets = mod_meta.get("target", [])
            if not manifest_targets:
                continue

            # Search for badge in the region after this heading (up to next heading)
            heading_end = heading_match.end()
            next_heading = re.search(r'^## ', content[heading_end:], re.MULTILINE)
            section_end = heading_end + next_heading.start() if next_heading else len(content)
            section_text = content[heading_end:section_end]

            badge_match = badge_pattern.search(section_text)
            if badge_match is None:
                inconsistencies.append(
                    f"{page_name}/{mod_name}: manifest has target "
                    f"{manifest_targets} but module page section lacks 🎯 Target badge"
                )
            else:
                badge_target = badge_match.group(1)
                for expected_target in manifest_targets:
                    if expected_target not in badge_target:
                        inconsistencies.append(
                            f"{page_name}/{mod_name}: target mismatch — "
                            f"manifest='{expected_target}', badge='{badge_target}'"
                        )

    if inconsistencies:
        errors.append(
            f"cross-page metadata inconsistency: "
            + "; ".join(inconsistencies)
            + "; regenerate with `python3 scripts/generate-docs.py`"
        )
        return 1

    return 0


def check_host_stub_fixture_coverage() -> int:
    """Verify that every host_stub function has at least one corresponding test fixture.

    For each function with ``kind = "host_stub"`` in the manifest, checks that
    a test fixture file in ``tests/fixtures/`` references the function name or
    its module.  This ensures CI catches untested host stubs before they
    silently bitrot.
    """
    if not MANIFEST.exists():
        return 0

    manifest = _tomllib.loads(MANIFEST.read_text(encoding="utf-8"))
    fixtures_dir = ROOT / "tests" / "fixtures"
    if not fixtures_dir.exists():
        return 0

    # Collect all fixture file contents for searching
    fixture_texts: dict[str, str] = {}
    for ark_file in fixtures_dir.rglob("*.ark"):
        try:
            fixture_texts[str(ark_file.relative_to(ROOT))] = ark_file.read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError):
            pass

    # Also consider fixture file *names* as coverage signals
    fixture_names = {p.stem for p in fixtures_dir.rglob("*.ark")}
    fixture_dirs = {p.name for p in fixtures_dir.iterdir() if p.is_dir()}

    uncovered: list[str] = []
    for fn in manifest.get("functions", []):
        if fn.get("kind") != "host_stub":
            continue
        name = fn["name"]
        module = fn.get("module", "")

        # Derive search terms: function name and module short name
        search_terms = [name]
        if "::" in module:
            # e.g. "std::host::sockets" → "sockets"
            short_module = module.rsplit("::", 1)[-1]
            search_terms.append(short_module)

        # Check 1: fixture file name contains the function name or module
        name_match = any(
            term.lower() in fname.lower()
            for term in search_terms
            for fname in fixture_names
        ) or any(
            term.lower() in dname.lower()
            for term in search_terms
            for dname in fixture_dirs
        )

        # Check 2: any fixture file body references the function or module
        body_match = any(
            term in content
            for term in search_terms
            for content in fixture_texts.values()
        )

        if not name_match and not body_match:
            uncovered.append(f"{name} (module={module})")

    if uncovered:
        errors.append(
            f"host_stub fixture coverage gap: {len(uncovered)} host_stub function(s) "
            f"lack test fixture coverage: {', '.join(uncovered)}; "
            "add fixture files in tests/fixtures/ that exercise these stubs"
        )
        return 1

    return 0


def check_stability_implementation_consistency() -> int:
    """Detect mismatches between stability metadata and implementation state.

    Checks for semantic inconsistencies such as:
    - A ``host_stub`` function marked ``stable`` (stubs should not be stable)
    - A ``deprecated`` function missing ``deprecated_by`` field
    - A function with ``deprecated_by`` not marked ``stability = "deprecated"``
    - Module-level stability in [[modules]] conflicting with function-level stability
    """
    if not MANIFEST.exists():
        return 0

    manifest = _tomllib.loads(MANIFEST.read_text(encoding="utf-8"))
    issues: list[str] = []

    # Build module stability lookup from [[modules]] entries
    module_stability: dict[str, str] = {}
    for mod in manifest.get("modules", []):
        mod_name = mod.get("name", "")
        mod_stab = mod.get("stability", "")
        if mod_name and mod_stab:
            module_stability[mod_name] = mod_stab

    for fn in manifest.get("functions", []):
        name = fn["name"]
        kind = fn.get("kind", "")
        stability = fn.get("stability", "")
        deprecated_by = fn.get("deprecated_by")
        module = fn.get("module", "")

        # 1. host_stub marked stable is suspicious — stubs are not production-ready
        if kind == "host_stub" and stability == "stable":
            issues.append(
                f"{name}: host_stub should not be 'stable' — "
                "use 'experimental' or 'provisional' until fully implemented"
            )

        # 2. deprecated_by set but stability is not "deprecated"
        if deprecated_by and stability != "deprecated":
            issues.append(
                f"{name}: has deprecated_by='{deprecated_by}' "
                f"but stability='{stability}' (expected 'deprecated')"
            )

        # 3. stability is "deprecated" but no deprecated_by
        if stability == "deprecated" and not deprecated_by:
            issues.append(
                f"{name}: stability='deprecated' but missing 'deprecated_by' field"
            )

        # 4. Function stability exceeds module stability
        #    e.g. function is "stable" but its module is "experimental"
        if module and module in module_stability:
            mod_stab = module_stability[module]
            stability_rank = {"experimental": 0, "provisional": 1, "stable": 2, "deprecated": -1}
            fn_rank = stability_rank.get(stability, -2)
            mod_rank = stability_rank.get(mod_stab, -2)
            if fn_rank > mod_rank and fn_rank >= 0 and mod_rank >= 0:
                issues.append(
                    f"{name}: function stability '{stability}' exceeds "
                    f"module '{module}' stability '{mod_stab}'"
                )

    if issues:
        errors.append(
            "stability-implementation inconsistency: "
            + "; ".join(issues)
        )
        return 1

    return 0


# ── Cookbook example drift detection (#401) ────────────────────────────────────

# Minimum overlap ratio between significant cookbook code lines and the union of
# fixture code lines.  Set permissively to allow simplified/combined examples
# while still catching real API drift (e.g. function renames, signature changes).
_COOKBOOK_DRIFT_THRESHOLD = 0.50

# Minimum number of significant lines (length > 3) for a recipe to be checked.
# Very short snippets are too small to measure overlap meaningfully.
_MIN_SIGNIFICANT_LINES = 3

# Language keywords and control-flow constructs that look like function calls
# (e.g. ``if (...)``, ``while (...)``) but should not be treated as API calls.
_LANG_KEYWORDS = frozenset({
    "fn", "if", "while", "match", "let", "return", "use", "else", "for",
    "enum", "struct", "impl", "pub", "mut", "const", "true", "false",
    "Ok", "Err", "Some", "None",
})


def _extract_api_calls(code: str) -> set[str]:
    """Extract function/method call names from Ark source code.

    Returns identifiers immediately preceding ``(``, excluding language
    keywords and locally-defined function names.  Module-qualified calls
    (``stdio::println``) are captured as a single token.
    """
    # Match: identifier (possibly module-qualified) followed by (
    calls = set(re.findall(
        r'\b([a-zA-Z_][a-zA-Z0-9_]*(?:::[a-zA-Z_][a-zA-Z0-9_]*)*)\s*\(', code
    ))
    calls -= _LANG_KEYWORDS
    # Remove locally defined functions (fn name(...))
    local_defs = set(re.findall(r'\bfn\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\(', code))
    calls -= local_defs
    return calls


def _normalize_code_lines(code: str) -> list[str]:
    """Normalize Ark source: strip inline comments, blank lines, whitespace.

    Returns a list of non-empty lines after removing trailing ``// …``
    comments and leading/trailing whitespace.  Used to compare cookbook
    code blocks against fixture source files.
    """
    result: list[str] = []
    for line in code.splitlines():
        # Strip trailing inline comments.  This is a simple approach that
        # does not handle // inside string literals, but the cookbook and
        # fixture code rarely contains // in strings.
        stripped = re.sub(r'\s*//.*$', '', line)
        stripped = stripped.strip()
        if stripped:
            result.append(stripped)
    return result


def _parse_cookbook_recipes(text: str) -> list[dict]:
    """Parse cookbook.md to extract recipes with fixture references and code blocks.

    Scans for ``## / ###`` headings, collects ``📎 Fixture`` link targets
    between headings and code fences, and associates them with the following
    `````ark`` code block.

    Returns a list of dicts:
      - ``heading``: the section heading text
      - ``fixture_paths``: list of fixture file paths (relative to repo root)
      - ``code``: the raw code block content
    """
    lines = text.splitlines()
    recipes: list[dict] = []
    current_heading = ""
    pending_fixtures: list[str] = []
    in_code_block = False
    code_lines: list[str] = []

    fixture_link_re = re.compile(r'\(\.\.\/\.\./(tests/fixtures/[^)]+\.ark)\)')

    for line in lines:
        # Track headings — each new heading resets pending fixtures
        if line.startswith("### "):
            current_heading = line[4:].strip()
            pending_fixtures = []
        elif line.startswith("## ") and not line.startswith("### "):
            current_heading = line[3:].strip()
            pending_fixtures = []

        # Collect fixture paths from any line outside code blocks
        if not in_code_block:
            for m in fixture_link_re.finditer(line):
                path = m.group(1)
                if path not in pending_fixtures:
                    pending_fixtures.append(path)

        # Code block start
        if line.strip().startswith("```ark"):
            in_code_block = True
            code_lines = []
            continue

        # Code block end
        if in_code_block and line.strip() == "```":
            in_code_block = False
            if pending_fixtures:
                recipes.append({
                    "heading": current_heading,
                    "fixture_paths": list(pending_fixtures),
                    "code": "\n".join(code_lines),
                })
                pending_fixtures = []
            continue

        if in_code_block:
            code_lines.append(line)

    return recipes


def check_cookbook_example_drift() -> int:
    """Verify that cookbook.md code blocks match their referenced fixture files.

    Performs two complementary checks for each recipe with ``📎 Fixture``
    references:

    1. **Line overlap** — extracts normalised code lines from the cookbook
       snippet and the referenced fixture files, and checks that at least
       ``_COOKBOOK_DRIFT_THRESHOLD`` of the cookbook's significant lines
       appear in the fixture union.  Catches major structural drift.

    2. **API-call consistency** — extracts function-call names from the
       cookbook snippet and checks that every call exists somewhere in the
       full fixture corpus (``tests/fixtures/**/*.ark``).  Catches stale
       function names after renames or removals.
    """
    cookbook_path = ROOT / "docs" / "stdlib" / "cookbook.md"
    if not cookbook_path.exists():
        return 0

    cookbook_text = cookbook_path.read_text(encoding="utf-8")
    recipes = _parse_cookbook_recipes(cookbook_text)

    if not recipes:
        return 0

    # ── Build corpus-wide API call set from all fixture files ─────────────
    fixtures_dir = ROOT / "tests" / "fixtures"
    corpus_api_calls: set[str] = set()
    if fixtures_dir.exists():
        for ark_file in fixtures_dir.rglob("*.ark"):
            try:
                corpus_api_calls |= _extract_api_calls(
                    ark_file.read_text(encoding="utf-8")
                )
            except (OSError, UnicodeDecodeError):
                pass

    drifted: list[str] = []
    checked = 0

    for recipe in recipes:
        heading = recipe["heading"]
        fixture_paths = recipe["fixture_paths"]
        code = recipe["code"]

        if not fixture_paths:
            continue

        # Union of normalised lines from all referenced fixtures
        fixture_lines: set[str] = set()
        fixtures_found = 0
        for fpath in fixture_paths:
            full_path = ROOT / fpath
            if full_path.exists():
                fixtures_found += 1
                ftext = full_path.read_text(encoding="utf-8")
                fixture_lines |= set(_normalize_code_lines(ftext))

        if not fixture_lines or fixtures_found == 0:
            continue

        # Normalised cookbook code lines
        cookbook_lines = _normalize_code_lines(code)

        # Keep only significant lines (length > 3) to avoid matching
        # trivial structural tokens like }, {, etc.
        significant = [l for l in cookbook_lines if len(l) > 3]

        if len(significant) < _MIN_SIGNIFICANT_LINES:
            continue

        checked += 1

        # ── Check 1: line overlap ─────────────────────────────────────────
        # Fraction of significant cookbook lines found in fixtures
        matched = sum(1 for l in significant if l in fixture_lines)
        ratio = matched / len(significant)

        if ratio < _COOKBOOK_DRIFT_THRESHOLD:
            unmatched = [l for l in significant if l not in fixture_lines]
            preview = "; ".join(unmatched[:3])
            if len(unmatched) > 3:
                preview += f" (+{len(unmatched) - 3} more)"
            drifted.append(
                f"'{heading}': {matched}/{len(significant)} significant lines "
                f"match fixtures ({ratio:.0%} < {_COOKBOOK_DRIFT_THRESHOLD:.0%}); "
                f"unmatched: {preview}"
            )
            continue  # skip API check — line-level already flagged

        # ── Check 2: API call consistency against full corpus ─────────────
        # Cookbook API calls that don't appear in ANY fixture file.
        # Catches stale function names after renames or removals.
        if corpus_api_calls:
            cookbook_api = _extract_api_calls(code)
            stale_calls = cookbook_api - corpus_api_calls
            if stale_calls:
                drifted.append(
                    f"'{heading}': cookbook uses API call(s) not found in any "
                    f"fixture: {', '.join(sorted(stale_calls))}"
                )

    if drifted:
        errors.append(
            f"cookbook example drift: {len(drifted)} recipe(s) have drifted "
            f"from their fixture files ({checked} checked):\n"
            + "\n".join(f"  \u2022 {d}" for d in drifted)
        )
        return 1

    return 0


def check_recipe_fixture_links() -> int:
    """Verify that recipe-manifest.toml fixture paths exist and match cookbook.md.

    Checks:
    - recipe-manifest.toml parses correctly
    - All recipe IDs are unique
    - Every fixture path listed in the manifest exists on disk
    - Every 📎 Fixture reference in cookbook.md appears in the manifest
    - Manifest fixture paths are a superset of cookbook references
    """
    manifest_path = ROOT / "docs" / "stdlib" / "recipe-manifest.toml"
    cookbook_path = ROOT / "docs" / "stdlib" / "cookbook.md"

    if not manifest_path.exists():
        errors.append(
            "recipe-manifest.toml does not exist; "
            "create docs/stdlib/recipe-manifest.toml with recipe-to-fixture mappings"
        )
        return 1

    if not cookbook_path.exists():
        return 0

    # ── 1. Parse manifest ─────────────────────────────────────────────────
    try:
        manifest = _tomllib.loads(manifest_path.read_text(encoding="utf-8"))
    except Exception as exc:
        errors.append(f"recipe-manifest.toml parse error: {exc}")
        return 1

    recipes = manifest.get("recipes", [])
    if not recipes:
        errors.append("recipe-manifest.toml has no [[recipes]] entries")
        return 1

    # ── 2. Check unique IDs ───────────────────────────────────────────────
    seen_ids: dict[str, int] = {}
    for recipe in recipes:
        rid = recipe.get("id", "")
        if rid in seen_ids:
            errors.append(f"recipe-manifest.toml: duplicate recipe id '{rid}'")
            return 1
        seen_ids[rid] = 1

    # ── 3. Check all fixture paths exist on disk ──────────────────────────
    missing_files: list[str] = []
    all_manifest_fixtures: set[str] = set()
    for recipe in recipes:
        rid = recipe.get("id", "")
        for fixture_path in recipe.get("fixtures", []):
            all_manifest_fixtures.add(fixture_path)
            full_path = ROOT / fixture_path
            if not full_path.exists():
                missing_files.append(f"{rid}: {fixture_path}")

    if missing_files:
        errors.append(
            f"recipe-manifest.toml has {len(missing_files)} broken fixture path(s): "
            + "; ".join(missing_files)
        )
        return 1

    # ── 4. Cross-check cookbook.md fixture references against manifest ─────
    cookbook_text = cookbook_path.read_text(encoding="utf-8")
    # Extract fixture paths from 📎 Fixture(s): lines
    # Pattern matches: tests/fixtures/…/file.ark inside []() markdown links
    cookbook_fixtures: set[str] = set()
    for match in re.finditer(
        r"\(\.\.\/\.\.\/(" r"tests/fixtures/[^)]+\.ark" r")\)", cookbook_text
    ):
        cookbook_fixtures.add(match.group(1))

    # Every cookbook fixture must appear in the manifest
    untracked = sorted(cookbook_fixtures - all_manifest_fixtures)
    if untracked:
        errors.append(
            f"recipe-manifest.toml missing {len(untracked)} cookbook fixture(s): "
            + "; ".join(untracked)
            + "; add them to docs/stdlib/recipe-manifest.toml"
        )
        return 1

    return 0


def check_name_index_completeness() -> int:
    """Verify that name-index.md contains every public function from manifest.

    Checks:
    - name-index.md exists and is non-empty
    - Every non-intrinsic function name from manifest appears in the index
    - Every deprecated function with deprecated_by appears in Historical section
    """
    name_index_path = ROOT / "docs" / "stdlib" / "name-index.md"
    if not name_index_path.exists():
        errors.append(
            "name-index.md does not exist; run `python3 scripts/generate-docs.py`"
        )
        return 1

    if not MANIFEST.exists():
        return 0

    manifest = _tomllib.loads(MANIFEST.read_text(encoding="utf-8"))
    functions = manifest.get("functions", [])
    public_functions = [
        entry for entry in functions
        if not entry.get("name", "").startswith("__intrinsic_")
    ]

    index_text = name_index_path.read_text(encoding="utf-8")
    missing: list[str] = []

    for entry in public_functions:
        name = entry["name"]
        # The name should appear in the index (either as `name` or ~~`name`~~)
        if f"`{name}`" not in index_text:
            missing.append(name)

    if missing:
        errors.append(
            f"name-index.md missing {len(missing)} function(s): "
            + ", ".join(missing[:10])
            + (f" (and {len(missing) - 10} more)" if len(missing) > 10 else "")
            + "; regenerate with `python3 scripts/generate-docs.py`"
        )
        return 1

    # Check deprecated entries appear in Historical section
    deprecated = [
        entry for entry in public_functions
        if entry.get("deprecated_by") or entry.get("stability") == "deprecated"
    ]
    if deprecated:
        if "## Historical / Deprecated Names" not in index_text:
            errors.append(
                "name-index.md missing '## Historical / Deprecated Names' section; "
                "regenerate with `python3 scripts/generate-docs.py`"
            )
            return 1

        # Verify each deprecated name appears with strikethrough in the historical section
        historical_section = index_text.split("## Historical / Deprecated Names")[1]
        historical_section = historical_section.split("## Combined Alphabetical Index")[0]
        missing_deprecated: list[str] = []
        for entry in deprecated:
            if f"~~`{entry['name']}`~~" not in historical_section:
                missing_deprecated.append(entry["name"])
        if missing_deprecated:
            errors.append(
                f"name-index.md historical section missing {len(missing_deprecated)} "
                f"deprecated name(s): {', '.join(missing_deprecated)}; "
                "regenerate with `python3 scripts/generate-docs.py`"
            )
            return 1

    return 0


def check_manifest_availability_consistency() -> int:
    """Validate that availability.t1=false when target is exclusively wasm32-wasi-p2.

    Functions that only list ``target = ["wasm32-wasi-p2"]`` and carry an
    ``[availability]`` block with ``t1 = true`` but no ``note`` are likely a
    data-entry mistake — ``t1`` should be ``false`` unless a ``note`` explains
    the T1 support path (e.g. via Wasmtime linker bridge).
    """
    if not MANIFEST.exists():
        return 0

    manifest = _tomllib.loads(MANIFEST.read_text(encoding="utf-8"))
    functions = manifest.get("functions", [])
    bad: list[str] = []

    for entry in functions:
        name = entry.get("name", "?")
        target = entry.get("target", [])
        avail = entry.get("availability")
        if target == ["wasm32-wasi-p2"] and avail is not None:
            # If t1 = true but no note is provided, this is a probable data error.
            # When a note is present, the author has explicitly documented the T1
            # support path (e.g. "T1 via Wasmtime linker"), which is a valid case.
            if avail.get("t1", True) is True and not avail.get("note"):
                bad.append(name)

    if bad:
        errors.append(
            f"availability.t1 should be false (or documented with a note) for "
            f"wasm32-wasi-p2-only functions: "
            + ", ".join(bad)
            + "; set availability.t1 = false or add an availability.note in std/manifest.toml"
        )
        return 1
    return 0


def check_manifest_example_integrity() -> int:
    """Validate that manifest examples have non-empty code fields.

    An ``[[functions.examples]]`` entry with an empty ``code`` string is a
    data-entry mistake — it would produce an empty code block in generated docs.
    """
    if not MANIFEST.exists():
        return 0

    manifest = _tomllib.loads(MANIFEST.read_text(encoding="utf-8"))
    functions = manifest.get("functions", [])
    bad: list[str] = []

    for entry in functions:
        name = entry.get("name", "?")
        for ex in entry.get("examples", []):
            if not ex.get("code", "").strip():
                bad.append(name)
                break

    if bad:
        errors.append(
            f"manifest has {len(bad)} function(s) with empty examples.code: "
            + ", ".join(bad)
            + "; fill in the code field in std/manifest.toml"
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
    failed += check_spec_guide_feature_drift()
    failed += check_target_metadata_in_reference()
    failed += check_stability_metadata_in_reference()
    failed += check_cross_page_metadata_consistency()
    failed += check_host_stub_fixture_coverage()
    failed += check_stability_implementation_consistency()
    failed += check_cookbook_example_drift()
    failed += check_recipe_fixture_links()
    failed += check_name_index_completeness()
    failed += check_manifest_availability_consistency()
    failed += check_manifest_example_integrity()

    if errors:
        print("docs consistency check FAILED:", file=sys.stderr)
        for err in errors:
            print(f"  ✗ {err}", file=sys.stderr)
        return 1

    print(f"docs consistency OK ({len(errors)} issues)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
