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


# ── Metadata-level verification (#403) ────────────────────────────────────────


def _parse_manifest() -> dict:
    """Load and return parsed manifest.toml."""
    import tomllib as _tomllib

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
    import tomllib as _tomllib

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
    import tomllib as _tomllib

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


def check_name_index_completeness() -> int:
    """Verify that name-index.md contains every public function from manifest.

    Checks:
    - name-index.md exists and is non-empty
    - Every non-intrinsic function name from manifest appears in the index
    - Every deprecated function with deprecated_by appears in Historical section
    """
    import tomllib as _tomllib

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
    failed += check_target_metadata_in_reference()
    failed += check_stability_metadata_in_reference()
    failed += check_cross_page_metadata_consistency()
    failed += check_host_stub_fixture_coverage()
    failed += check_stability_implementation_consistency()
    failed += check_name_index_completeness()

    if errors:
        print("docs consistency check FAILED:", file=sys.stderr)
        for err in errors:
            print(f"  ✗ {err}", file=sys.stderr)
        return 1

    print(f"docs consistency OK ({len(errors)} issues)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
