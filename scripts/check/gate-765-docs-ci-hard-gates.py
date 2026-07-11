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
BENCHMARK_RESULTS = REPO / "docs" / "process" / "benchmark-results.md"
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
    if count is None:
        failures.append("project-state.toml missing fixture_manifest_count")
        return
    harness = f"{passed} passed, {failed} failed, {skipped} skipped"
    with_total = f"{harness} / {count} entries"
    views = [
        (REPO / "README.md", with_total),
        (REPO / "docs" / "README.md", with_total),
        (REPO / "docs" / "language" / "README.md", f"{count} manifest entries"),
        (REPO / "docs" / "process" / "README.md", f"{count} entries"),
        (REPO / "docs" / "current-state.md", f"Fixture manifest: {count} entries"),
    ]
    for path, needle in views:
        if not path.is_file():
            failures.append(f"missing view {rel(path)}")
            continue
        text = path.read_text(encoding="utf-8")
        if needle not in text:
            failures.append(f"numeric drift in {rel(path)}: expected to contain {needle!r}")
        # Root README views must include failures
        if path in {REPO / "README.md", REPO / "docs" / "README.md"}:
            if failed is not None and f"{failed} failed" not in text:
                failures.append(f"{rel(path)} must display fixture failures ({failed})")


def check_deprecated_vocab(cfg: dict, failures: list[str]) -> None:
    dep = cfg.get("deprecated_vocab", {})
    globs = dep.get("path_globs", [])
    allow_sub = [s.lower() for s in dep.get("line_allow_substrings", [])]
    strict_patterns = dep.get("strict_paths", [])
    files = expand_globs(strict_patterns)
    # Also include language/platform/state READMEs via globs already
    for path in sorted({p for p in files if p.is_file() and p.suffix == ".md"}):
        if path_allowed(path, globs):
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
        failures.append("missing process/benchmark-results.md")
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
    budgets = cfg.get("skip_doc_check", {}).get("budgets", [])
    for entry in budgets:
        path = REPO / entry["path"]
        max_n = int(entry["max"])
        if not path.is_file():
            failures.append(f"skip budget path missing: {entry['path']}")
            continue
        text = path.read_text(encoding="utf-8")
        matches = list(SKIP_RE.finditer(text))
        n = len(matches)
        if n > max_n:
            failures.append(
                f"skip-doc-check budget exceeded in {entry['path']}: {n} > {max_n}"
            )
        # New structured form is encouraged; unstructured still allowed within budget.
        # If attrs present, require structured keys.
        for m in matches:
            attrs = (m.group(1) or "").strip()
            if not attrs:
                continue
            if "reason=" in attrs and not STRUCTURED_SKIP.search(attrs):
                failures.append(
                    f"structured skip-doc-check malformed in {entry['path']}: {attrs[:80]}"
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

    if failures:
        print("gate-765-docs-ci-hard-gates: FAIL", file=sys.stderr)
        for item in failures:
            print(f"  - {item}", file=sys.stderr)
        return 1

    print("gate-765-docs-ci-hard-gates: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
