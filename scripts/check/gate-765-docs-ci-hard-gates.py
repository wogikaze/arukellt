#!/usr/bin/env python3
"""Docs CI hard gates for issue #765.

Enforces re-occurrence prevention after the 2026-07-11 docs audit:
numeric consistency, deprecated vocabulary, ownership, classification,
benchmark validity, overview archive posture, capability axes, orphans,
section registry parity, and skip-doc-check budgets.
"""

from __future__ import annotations

import re
import sys
from datetime import date
from fnmatch import fnmatch
from pathlib import Path

try:
    import tomllib
except ImportError:  # Python < 3.11
    import tomli as tomllib  # type: ignore

REPO = Path(__file__).resolve().parents[2]
CONFIG = REPO / "docs" / "data" / "docs-gate-config.toml"
STATE = REPO / "docs" / "data" / "project-state.toml"
SECTIONS = REPO / "docs" / "data" / "sections.toml"
CLASSIFICATIONS = REPO / "docs" / "data" / "language-doc-classifications.toml"
OVERVIEW = REPO / "docs" / "overview.html"
INDEX_HTML = REPO / "docs" / "index.html"
CAPABILITY = REPO / "docs" / "capability-surface.md"
BENCHMARK_RESULTS = REPO / "docs" / "history" / "benchmarks" / "benchmark-results.md"
OWNERSHIP = REPO / "docs" / "directory-ownership.md"
GENERATED_MANIFEST = REPO / ".generated-files"

DEPRECATED_NAME = re.compile(r"\bwasm32-wasi-p[123]\b|\bwasm32-freestanding\b")
# Target-tier labels (not type parameters like T1 in (T1, T2))
DEPRECATED_TIER = re.compile(
    r"(?:`T[0-5]`|\(T[0-5]\)|\bT[0-5]\s+(?:backend|vs|/)|"
    r"\bT[0-5]系|\bT[0-5]\s*\([^)]*(?:wasm|WASI|GC|linear))",
    re.IGNORECASE,
)
SKIP_RE = re.compile(r"<!--\s*skip-doc-check(?:\s+([^>]*))?-->", re.IGNORECASE)
STRUCTURED_SKIP = re.compile(
    r'reason\s*=\s*"[^"]+"\s+owner\s*=\s*"[^"]+"\s+kind\s*=\s*"(?:pseudocode|future|non-runnable)"'
    r'(?:\s+expires\s*=\s*"\d{4}-\d{2}-\d{2}")?'
)


def load_toml(path: Path) -> dict:
    return tomllib.loads(path.read_text(encoding="utf-8"))


def rel(path: Path) -> str:
    return path.relative_to(REPO).as_posix()


def expand_globs(patterns: list[str]) -> list[Path]:
    out: list[Path] = []
    for pattern in patterns:
        if any(ch in pattern for ch in "*?["):
            # Manual glob from repo root
            parts = pattern.split("/")
            # Use Path.glob for recursive **
            out.extend(REPO.glob(pattern))
        else:
            p = REPO / pattern
            if p.exists():
                out.append(p)
    return out


def path_allowed(path: Path, globs: list[str]) -> bool:
    r = rel(path)
    return any(fnmatch(r, g) for g in globs)


def check_numeric_consistency(state: dict, failures: list[str]) -> None:
    verification = state.get("verification", {})
    count = verification.get("fixture_manifest_count")
    passed = verification.get("fixture_passed")
    failed = verification.get("fixture_failures")
    skipped = verification.get("fixture_skipped")
    observed = verification.get("fixture_harness_observed")
    remainder = verification.get("fixture_not_in_last_harness_snapshot")
    if count is None:
        failures.append("project-state.toml missing fixture_manifest_count")
        return
    if passed is None or failed is None or skipped is None:
        failures.append("project-state.toml missing fixture_passed/failures/skipped")
        return
    sum_outcomes = int(passed) + int(failed) + int(skipped)
    if observed is None:
        failures.append("project-state.toml missing fixture_harness_observed")
    elif int(observed) != sum_outcomes:
        failures.append(
            f"fixture_harness_observed={observed} != passed+failed+skipped={sum_outcomes}"
        )
    if remainder is None:
        failures.append("project-state.toml missing fixture_not_in_last_harness_snapshot")
    elif observed is not None and int(remainder) != int(count) - int(observed):
        failures.append(
            f"fixture_not_in_last_harness_snapshot={remainder} != "
            f"manifest-observed={int(count) - int(observed)}"
        )
    harness = f"{passed} passed, {failed} failed, {skipped} skipped (observed harness: {observed})"
    with_registry = f"{harness}; registry: {count} manifest entries"
    views = [
        (REPO / "README.md", with_registry),
        (REPO / "docs" / "README.md", with_registry),
        (REPO / "docs" / "language" / "README.md", f"{count} manifest entries"),
        (REPO / "docs" / "process" / "README.md", f"{count} manifest entries"),
        (REPO / "docs" / "current-state.md", f"Fixture registry: {count} manifest entries"),
    ]
    for path, needle in views:
        if not path.is_file():
            failures.append(f"missing view {rel(path)}")
            continue
        text = path.read_text(encoding="utf-8")
        if needle not in text:
            failures.append(f"numeric drift in {rel(path)}: expected to contain {needle!r}")
        if path in {REPO / "README.md", REPO / "docs" / "README.md"}:
            if failed is not None and f"{failed} failed" not in text:
                failures.append(f"{rel(path)} must display fixture failures ({failed})")
            if "observed harness" not in text:
                failures.append(f"{rel(path)} must label harness totals as observed (not same unit as registry)")
            if "registry:" not in text.lower() and f"{count} manifest entries" not in text:
                failures.append(f"{rel(path)} must display registry size separately from harness")


def adr_is_accepted(text: str, markers: list[str]) -> bool:
    head = "\n".join(text.splitlines()[:60])
    low = head.lower()
    if "accepted" in low and ("ステータス" in head or "status" in low):
        # Prefer explicit markers when provided
        for m in markers:
            if m.lower() in low or m in head:
                return True
        if re.search(r"\bACCEPTED\b", head):
            return True
    return False


def check_deprecated_vocab(cfg: dict, failures: list[str]) -> None:
    dep = cfg.get("deprecated_vocab", {})
    globs = dep.get("path_globs", [])
    allow_sub = [s.lower() for s in dep.get("line_allow_substrings", [])]
    strict_patterns = dep.get("strict_paths", [])
    files = expand_globs(strict_patterns)
    adr_cfg = cfg.get("accepted_adr", {})
    markers = adr_cfg.get("accepted_status_markers", [])
    # Also include language/platform/state READMEs via globs already
    for path in sorted({p for p in files if p.is_file() and p.suffix == ".md"}):
        if path_allowed(path, globs):
            continue
        # ADR special case: only Accepted ADRs are in scope (#770).
        if "docs/adr/" in rel(path).replace("\\", "/"):
            text_probe = path.read_text(encoding="utf-8")
            if not adr_is_accepted(text_probe, markers):
                continue
        text = path.read_text(encoding="utf-8")
        for i, line in enumerate(text.splitlines(), 1):
            low = line.lower()
            if any(s in low or s in line for s in allow_sub):
                continue
            if DEPRECATED_NAME.search(line) or DEPRECATED_TIER.search(line):
                failures.append(
                    f"deprecated vocab in {rel(path)}:{i}: {line.strip()[:140]}"
                )


def check_classification(failures: list[str]) -> None:
    if not CLASSIFICATIONS.is_file():
        failures.append("missing language-doc-classifications.toml")
        return
    data = load_toml(CLASSIFICATIONS)
    classified = {entry["file"] for entry in data.get("docs", []) if "file" in entry}
    lang_dir = REPO / "docs" / "language"
    actual = {p.name for p in lang_dir.glob("*.md") if p.name != "README.md"}
    missing = sorted(actual - classified)
    if missing:
        failures.append(
            "language docs unclassified (add to language-doc-classifications.toml): "
            + ", ".join(missing)
        )


def check_ownership(cfg: dict, failures: list[str]) -> None:
    own = cfg.get("ownership", {})
    must_ssot = own.get("must_be_hand_ssot", [])
    if not OWNERSHIP.is_file():
        failures.append("missing directory-ownership.md")
        return
    text = OWNERSHIP.read_text(encoding="utf-8")
    # Directory-level claim that docs/data/ is entirely generated is forbidden
    for line in text.splitlines():
        if re.search(r"`docs/data/`\s*\|\s*generated", line):
            failures.append(
                "directory-ownership.md must not mark docs/data/ wholesale as generated"
            )
    for path in must_ssot:
        if path not in text:
            failures.append(f"directory-ownership.md must document SSOT input {path}")
        # Look for a row that marks it generated incorrectly
        for line in text.splitlines():
            if path in line and "| generated |" in line.replace("**", ""):
                # allow target-contract-summary
                if "target-contract-summary" in path:
                    continue
                if "hand" in line or "SSOT" in line or "hand-maintained" in line:
                    continue
                failures.append(f"{path} appears classified as generated in ownership map")
    if GENERATED_MANIFEST.is_file():
        gen_text = GENERATED_MANIFEST.read_text(encoding="utf-8")
        for path in must_ssot:
            for line in gen_text.splitlines():
                if line.startswith("#") or "|" not in line:
                    continue
                if line.split("|", 1)[0].strip() == path:
                    failures.append(f"{path} must not be listed in .generated-files")


def check_benchmark_validity(failures: list[str]) -> None:
    if not BENCHMARK_RESULTS.is_file():
        failures.append("missing history/benchmarks/benchmark-results.md")
        return
    text = BENCHMARK_RESULTS.read_text(encoding="utf-8")
    head = "\n".join(text.splitlines()[:20]).lower()
    invalid = "invalid" in head or "no measurements" in head
    # Detect all-skipped / deprecated target current run
    current = text
    if "## Current Run" in text:
        current = text.split("## Current Run", 1)[1][:2000]
    bad_target = "wasm32-wasi-p1" in current or "wasm32-wasi-p2" in current
    # Count skipped-looking rows in a simple way
    skippedish = current.lower().count("skipped") + current.lower().count("n/a")
    if (bad_target or skippedish >= 5) and not invalid:
        failures.append(
            "benchmark-results.md current run looks invalid but lacks INVALID banner"
        )
    # current-state must not present it as live evidence
    cs = (REPO / "docs" / "current-state.md").read_text(encoding="utf-8")
    if "Performance Snapshot" in cs:
        block = cs.split("Performance Snapshot", 1)[1][:1200].lower()
        if "invalid" not in block and "no current measurements" not in block:
            if "benchmark-results.md" in block:
                failures.append(
                    "current-state Performance Snapshot must mark benchmark-results as INVALID"
                )


def check_overview(cfg: dict, failures: list[str]) -> None:
    ov = cfg.get("overview", {})
    if not OVERVIEW.is_file():
        failures.append("missing docs/overview.html")
        return
    text = OVERVIEW.read_text(encoding="utf-8")
    # Banner must appear in/near <body>, not only in comments deep in the file.
    body_idx = text.lower().find("<body")
    window = text[body_idx : body_idx + 1500] if body_idx >= 0 else text[:2500]
    banners = ov.get("require_archive_banner_substrings", [])
    if not any(b in window for b in banners):
        failures.append("overview.html missing archive/stale banner near <body>")
    # Must not be sold as primary entry without archive label
    for path_s in ov.get("forbid_as_primary_entry_in", []):
        path = REPO / path_s
        if not path.is_file():
            continue
        body = path.read_text(encoding="utf-8")
        for line in body.splitlines():
            if "overview.html" not in line:
                continue
            low = line.lower()
            if "アーカイブ" in line or "archiv" in low or "正本ではない" in line:
                continue
            if "初見" in line or "全体マップ" in line or "one pager" in low:
                failures.append(
                    f"{path_s} still presents overview.html as a primary current entry: {line.strip()[:120]}"
                )


def check_capability(cfg: dict, failures: list[str]) -> None:
    cap = cfg.get("capability", {})
    if not CAPABILITY.is_file():
        failures.append("missing capability-surface.md")
        return
    text = CAPABILITY.read_text(encoding="utf-8")
    low = text.lower()
    for axis in cap.get("require_axes", []):
        if axis.lower() not in low:
            failures.append(f"capability-surface.md missing axis {axis!r}")
    for phrase in cap.get("forbid_phrases", []):
        if phrase in text:
            failures.append(
                "capability-surface.md still overclaims with unified availability phrasing"
            )
    # Ban a lone available bool table header
    if re.search(r"\|\s*available\s*\|", text, re.I) and "user_reachable" not in low:
        failures.append("capability-surface.md must not use a single available column")


def check_orphans(cfg: dict, failures: list[str]) -> None:
    required = cfg.get("orphan_current", {}).get("required_inbound", [])
    # Build corpus of markdown link targets from key indexes
    indexes = [
        REPO / "docs" / "README.md",
        REPO / "docs" / "_sidebar.md",
        REPO / "README.md",
        REPO / "docs" / "current-state.md",
    ]
    blob = "\n".join(p.read_text(encoding="utf-8") for p in indexes if p.is_file())
    for req in required:
        name = Path(req).name
        # accept several link forms
        ok = (
            name in blob
            or req in blob
            or req.removeprefix("docs/") in blob
            or f"/{name}" in blob
        )
        if not ok:
            failures.append(f"orphan current doc (no inbound from indexes): {req}")


def check_section_registry_parity(failures: list[str]) -> None:
    if not SECTIONS.is_file() or not INDEX_HTML.is_file():
        failures.append("missing sections.toml or docs/index.html")
        return
    sections = load_toml(SECTIONS).get("sections", [])
    section_dirs = {s["dir"] for s in sections if "dir" in s}
    html = INDEX_HTML.read_text(encoding="utf-8")
    m = re.search(r"const rootRoutes = new Set\(\[(.*?)\]\)", html, re.S)
    if not m:
        failures.append("docs/index.html missing rootRoutes Set")
        return
    routes = set(re.findall(r"'([^']+)'", m.group(1)))
    # Section dirs must be present in rootRoutes
    missing = sorted(section_dirs - routes)
    if missing:
        failures.append(
            "Docsify rootRoutes missing section dirs from sections.toml: "
            + ", ".join(missing)
        )
    sidebar = (REPO / "docs" / "_sidebar.md").read_text(encoding="utf-8")
    for d in sorted(section_dirs):
        if f"#/{d}/" not in sidebar and f"#/{d}/README" not in sidebar:
            failures.append(f"_sidebar.md missing section {d}")


def check_skip_budget(cfg: dict, failures: list[str]) -> None:
    skip_cfg = cfg.get("skip_doc_check", {})
    budgets = skip_cfg.get("budgets", [])
    require_structured = bool(skip_cfg.get("require_structured", False))
    require_expires = bool(skip_cfg.get("require_expires", False))
    global_max = skip_cfg.get("global_max")
    global_ratchet = skip_cfg.get("global_ratchet_max")
    require_all_budgeted = bool(skip_cfg.get("require_all_budgeted", False))
    budgeted_paths = set()
    total_count = 0
    for entry in budgets:
        path = REPO / entry["path"]
        budgeted_paths.add(entry["path"])
        max_n = int(entry["max"])
        ratchet = entry.get("ratchet_max")
        if ratchet is not None and max_n > int(ratchet):
            failures.append(
                f"skip-doc-check budget ratchet violated in {entry['path']}: "
                f"max={max_n} > ratchet_max={ratchet} (budgets may only decrease)"
            )
        if not path.is_file():
            failures.append(f"skip budget path missing: {entry['path']}")
            continue
        text = path.read_text(encoding="utf-8")
        matches = list(SKIP_RE.finditer(text))
        n = len(matches)
        total_count += n
        if n > max_n:
            failures.append(
                f"skip-doc-check budget exceeded in {entry['path']}: {n} > {max_n}"
            )
        for m in matches:
            attrs = (m.group(1) or "").strip()
            if require_structured:
                if not attrs or not STRUCTURED_SKIP.search(attrs):
                    failures.append(
                        f"unstructured skip-doc-check in {entry['path']}: "
                        f"require reason/owner/kind"
                        + ("/expires" if require_expires else "")
                    )
                    break
                if require_expires and "expires=" not in attrs:
                    failures.append(
                        f"skip-doc-check missing expires= in {entry['path']}"
                    )
                    break
            elif attrs:
                if "reason=" in attrs and not STRUCTURED_SKIP.search(attrs):
                    failures.append(
                        f"structured skip-doc-check malformed in {entry['path']}: {attrs[:80]}"
                    )

    if not require_structured:
        return

    # Global ceiling check
    if global_max is not None and total_count > int(global_max):
        failures.append(
            f"skip-doc-check global ceiling exceeded: {total_count} > {global_max}"
        )
    if global_ratchet is not None and global_max is not None and int(global_max) > int(global_ratchet):
        failures.append(
            f"skip-doc-check global ratchet violated: global_max={global_max} > "
            f"global_ratchet_max={global_ratchet} (ceiling may only decrease)"
        )

    # Unbudgeted file detection
    if require_all_budgeted:
        for path in sorted((REPO / "docs").rglob("*.md")):
            rel_path = rel(path)
            text = path.read_text(encoding="utf-8")
            matches = list(SKIP_RE.finditer(text))
            if matches and rel_path not in budgeted_paths:
                failures.append(
                    f"skip-doc-check in unbudgeted file: {rel_path} ({len(matches)} skips, "
                    f"no budget entry in docs-gate-config.toml)"
                )

    for path in sorted((REPO / "docs").rglob("*.md")):
        for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            for match in SKIP_RE.finditer(line):
                attrs = (match.group(1) or "").strip()
                if not STRUCTURED_SKIP.fullmatch(attrs):
                    failures.append(
                        f"unstructured skip-doc-check in {rel(path)}:{line_number}: "
                        "require reason/owner/kind/expires"
                    )
                    continue
                owner_match = re.search(r'owner="([^"]+)"', attrs)
                if not owner_match or not re.fullmatch(r"#\d+", owner_match.group(1)):
                    failures.append(
                        f"unknown skip-doc-check owner in {rel(path)}:{line_number}: "
                        f"{owner_match.group(1) if owner_match else 'missing'}"
                    )
                expiry_match = re.search(r'expires="(\d{4}-\d{2}-\d{2})"', attrs)
                if require_expires and not expiry_match:
                    failures.append(f"skip-doc-check missing expires= in {rel(path)}:{line_number}")
                elif expiry_match:
                    try:
                        expiry = date.fromisoformat(expiry_match.group(1))
                    except ValueError:
                        failures.append(f"invalid skip-doc-check expiry in {rel(path)}:{line_number}")
                    else:
                        if expiry < date.today():
                            failures.append(
                                f"expired skip-doc-check in {rel(path)}:{line_number}: {expiry}"
                            )


def check_target_contract_summary_generated(failures: list[str]) -> None:
    path = REPO / "docs" / "data" / "target-contract-summary.md"
    if not path.is_file():
        failures.append("missing target-contract-summary.md")
        return
    text = path.read_text(encoding="utf-8")
    if "Generated" not in text.splitlines()[0:6].__str__() and "**Generated**" not in text[:400]:
        failures.append("target-contract-summary.md must declare Generated from project-state.toml")


def check_bootstrap_contract(cfg: dict, failures: list[str]) -> None:
    boot = cfg.get("bootstrap", {})
    forbids = boot.get("forbid_substrings", [])
    for rel_path in boot.get("scan_paths", []):
        path = REPO / rel_path
        if not path.is_file():
            failures.append(f"missing bootstrap scan path {rel_path}")
            continue
        text = path.read_text(encoding="utf-8")
        for needle in forbids:
            if needle in text:
                failures.append(f"Rust-era bootstrap wording in {rel_path}: {needle!r}")


def check_cli_binary_name(cfg: dict, failures: list[str]) -> None:
    cli = cfg.get("cli", {})
    patterns = cli.get("forbid_patterns", [])
    for rel_path in cli.get("scan_paths", []):
        path = REPO / rel_path
        if not path.is_file():
            continue
        text = path.read_text(encoding="utf-8")
        for pat in patterns:
            if pat in text:
                failures.append(
                    f"undefined `ark` CLI alias in {rel_path}: found {pat!r} (use arukellt)"
                )


def check_ci_job_ids(cfg: dict, failures: list[str]) -> None:
    ci = cfg.get("ci_jobs", {})
    gen_rel = ci.get("generated_doc", "docs/data/ci-jobs.md")
    gen_path = REPO / gen_rel
    wf_path = REPO / ci.get("workflow", ".github/workflows/ci.yml")
    if not wf_path.is_file():
        failures.append("missing ci.yml for job taxonomy")
        return
    body = wf_path.read_text(encoding="utf-8").split("jobs:", 1)[-1]
    real_jobs = set(re.findall(r"^  ([A-Za-z0-9_-]+):\s*$", body, re.M))
    if gen_path.is_file():
        gen_text = gen_path.read_text(encoding="utf-8")
        for job in sorted(real_jobs):
            if f"`{job}`" not in gen_text:
                failures.append(f"{gen_rel} missing job id `{job}` — regenerate ci-jobs doc")
    else:
        failures.append(f"missing {gen_rel}; run scripts/gen/generate-ci-jobs-doc.py")
    forbidden = ci.get("forbidden_job_ids", [])
    for rel_path in ci.get("scan_paths", []):
        path = REPO / rel_path
        if not path.is_file():
            continue
        text = path.read_text(encoding="utf-8")
        # Skip multi-line "do not invent / historical names" blocks.
        skip_block = False
        for i, line in enumerate(text.splitlines(), 1):
            low = line.lower()
            if "do **not** invent" in low or "do not invent" in low or "historical / incorrect" in low:
                skip_block = True
            if skip_block:
                if line.strip() == "" or line.startswith("##"):
                    skip_block = False
                else:
                    continue
            for job in forbidden:
                if not re.search(rf"`{re.escape(job)}`", line):
                    continue
                if any(
                    k in low
                    for k in (
                        "must not",
                        "incorrect",
                        "historical",
                        "not a top-level",
                        "no dedicated",
                        "unknown",
                        "not** a",
                        "include:",
                    )
                ):
                    continue
                failures.append(
                    f"unknown/historical CI job id `{job}` in {rel_path}:{i}: {line.strip()[:100]}"
                )


def check_capability_policy(cfg: dict, failures: list[str]) -> None:
    pol = cfg.get("capability_policy", {})
    path = REPO / pol.get("path", "docs/process/policy.md")
    if not path.is_file():
        failures.append("missing process/policy.md")
        return
    text = path.read_text(encoding="utf-8")
    for needle in pol.get("forbid_substrings", []):
        if needle in text:
            failures.append(f"stale capability policy wording in {rel(path)}: {needle!r}")


def check_structured_state(failures: list[str]) -> None:
    """Phase-2 SSOTs and generated views must exist (#770)."""
    required_tomls = [
        "docs/data/cli-surface.toml",
        "docs/data/bootstrap-contract.toml",
        "docs/data/capabilities.toml",
        "docs/data/component-availability.toml",
        "docs/data/release-guarantees.toml",
    ]
    required_views = [
        "docs/capability-surface.md",
        "docs/data/cli-surface.md",
        "docs/data/bootstrap-contract.md",
        "docs/data/component-availability.md",
        "docs/data/release-guarantees.md",
    ]
    for rel_path in required_tomls + required_views:
        if not (REPO / rel_path).is_file():
            failures.append(f"missing structured state artifact: {rel_path}")
    cap = REPO / "docs" / "capability-surface.md"
    if cap.is_file():
        text = cap.read_text(encoding="utf-8")
        if "Generated" not in text[:500] or "capabilities.toml" not in text[:800]:
            failures.append(
                "capability-surface.md must be generated from docs/data/capabilities.toml"
            )
    readme = REPO / "docs" / "README.md"
    if readme.is_file():
        text = readme.read_text(encoding="utf-8")
        if "multi-axis" not in text and "`command_component`" not in text:
            failures.append(
                "docs/README.md must not flatten component emit to available true/false"
            )


def check_release_guarantees(failures: list[str]) -> None:
    data = load_toml(REPO / "docs/data/release-guarantees.toml")
    checklist = (REPO / "docs/release-checklist.md").read_text(encoding="utf-8")
    workflow = (REPO / ".github/workflows/ci.yml").read_text(encoding="utf-8")
    workflow_jobs = set(re.findall(r"^  ([A-Za-z0-9_-]+):\s*$", workflow.split("jobs:", 1)[-1], re.M))
    commands: set[str] = set()
    for guarantee in data.get("guarantees", []):
        if not guarantee.get("release_blocker"):
            continue
        check = guarantee.get("check", "").strip()
        job = guarantee.get("ci_job", "").strip()
        if not re.match(r"^(?:python3?|bash|scripts/)", check):
            failures.append(f"release blocker {guarantee['id']} lacks an exact executable command: {check!r}")
        if check in commands:
            failures.append(f"release blockers reuse generic command: {check}")
        commands.add(check)
        if job not in workflow_jobs:
            failures.append(f"release blocker {guarantee['id']} references unknown CI job: {job}")
        expected = f"**CI `{guarantee['id']}`** — `{check}` (job: `{job}`)"
        if expected not in checklist:
            failures.append(f"release checklist missing generated blocker row: {guarantee['id']}")


def main() -> int:
    failures: list[str] = []
    if not CONFIG.is_file():
        print("FAIL: missing docs/data/docs-gate-config.toml", file=sys.stderr)
        return 1
    if not STATE.is_file():
        print("FAIL: missing docs/data/project-state.toml", file=sys.stderr)
        return 1

    cfg = load_toml(CONFIG)
    state = load_toml(STATE)

    check_numeric_consistency(state, failures)
    check_deprecated_vocab(cfg, failures)
    check_classification(failures)
    check_ownership(cfg, failures)
    check_benchmark_validity(failures)
    check_overview(cfg, failures)
    check_capability(cfg, failures)
    check_orphans(cfg, failures)
    check_section_registry_parity(failures)
    check_skip_budget(cfg, failures)
    check_target_contract_summary_generated(failures)
    check_bootstrap_contract(cfg, failures)
    check_cli_binary_name(cfg, failures)
    check_ci_job_ids(cfg, failures)
    check_capability_policy(cfg, failures)
    check_structured_state(failures)
    check_release_guarantees(failures)

    if failures:
        print("gate-765-docs-ci-hard-gates: FAIL", file=sys.stderr)
        for item in failures:
            print(f"  - {item}", file=sys.stderr)
        return 1

    print("gate-765-docs-ci-hard-gates: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
