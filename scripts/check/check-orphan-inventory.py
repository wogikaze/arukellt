#!/usr/bin/env python3
"""Orphan / stale file inventory (advisory — always exits 0).

Scans docs, tests, benchmarks, and committed artifact-like paths for:
  1. large files (>500KB)
  2. orphan fixtures (.ark not listed in manifest.txt)
  3. orphan .expected (no corresponding .ark source)
  4. broken doc refs (markdown links in docs/ to missing files)
  5. orphan bench assets (.ark not in benchmark registry or manifest bench entries)

Issue #418. Reports candidates and reference status; does not fail CI.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]

LARGE_FILE_THRESHOLD = 500 * 1024  # docs/retention-policy.md audit threshold
SCAN_ROOTS = ("docs", "tests", "benchmarks")
ARTIFACT_SUBDIRS = ("benchmarks/baselines", "benchmarks/results")

EXCLUDE_DIR_NAMES = {
    ".git",
    "node_modules",
    "target",
    ".vscode-test",
    "__pycache__",
}

LINK_REF_RE = re.compile(r"\]\(([^)]+)\)")
BENCH_SOURCE_RE = re.compile(r"source=\"([^\"]+)\"")
BENCH_EXPECTED_RE = re.compile(r"expected=\"([^\"]+)\"")


def _import_fixture_helpers() -> tuple:
    sys.path.insert(0, str(REPO_ROOT / "scripts"))
    from verify.fixtures import disk_fixture_paths, load_manifest

    return disk_fixture_paths, load_manifest


def _is_excluded(path: Path) -> bool:
    return any(part in EXCLUDE_DIR_NAMES for part in path.parts)


def _rel(path: Path) -> str:
    return str(path.relative_to(REPO_ROOT)).replace("\\", "/")


def _scan_large_files() -> list[tuple[str, int, str]]:
    findings: list[tuple[str, int, str]] = []
    roots: list[Path] = [REPO_ROOT / name for name in SCAN_ROOTS]
    for sub in ARTIFACT_SUBDIRS:
        candidate = REPO_ROOT / sub
        if candidate.is_dir():
            roots.append(candidate)

    seen: set[str] = set()
    for root in roots:
        if not root.is_dir():
            continue
        for path in root.rglob("*"):
            if not path.is_file() or _is_excluded(path):
                continue
            rel = _rel(path)
            if rel in seen:
                continue
            seen.add(rel)
            size = path.stat().st_size
            if size > LARGE_FILE_THRESHOLD:
                findings.append(
                    (rel, size, "not referenced (size audit only)")
                )
    return sorted(findings, key=lambda item: (-item[1], item[0]))


def _scan_orphan_fixtures() -> list[tuple[str, str]]:
    fixtures_root = REPO_ROOT / "tests" / "fixtures"
    manifest_path = fixtures_root / "manifest.txt"
    if not manifest_path.is_file():
        return []

    disk_fixture_paths, load_manifest = _import_fixture_helpers()
    manifest_paths = {
        entry["path"]
        for entry in load_manifest(manifest_path)
        if entry["kind"] != "bench"
    }
    disk_paths = set(disk_fixture_paths(fixtures_root))

    findings: list[tuple[str, str]] = []
    for rel_path in sorted(disk_paths - manifest_paths):
        findings.append((f"tests/fixtures/{rel_path}", "not in manifest.txt"))
    return findings


def _expected_source_candidates(expected_path: Path) -> list[Path]:
    stem = expected_path.with_suffix("")
    candidates = [stem.with_suffix(".ark")]
    parent_main = expected_path.parent / "main.ark"
    if parent_main.is_file():
        candidates.append(parent_main)
    return candidates


def _scan_orphan_expected() -> list[tuple[str, str]]:
    findings: list[tuple[str, str]] = []
    search_roots = [
        REPO_ROOT / "tests" / "fixtures",
        REPO_ROOT / "benchmarks",
    ]
    for root in search_roots:
        if not root.is_dir():
            continue
        for path in sorted(root.rglob("*.expected")):
            if _is_excluded(path):
                continue
            if any(c.is_file() for c in _expected_source_candidates(path)):
                continue
            findings.append((_rel(path), "no matching .ark source"))
    return findings


def _scan_broken_doc_refs() -> list[tuple[str, str]]:
    findings: list[tuple[str, str]] = []
    docs_root = REPO_ROOT / "docs"
    if not docs_root.is_dir():
        return findings

    for md_path in sorted(docs_root.rglob("*.md")):
        if _is_excluded(md_path):
            continue
        text = md_path.read_text(encoding="utf-8")
        md_dir = md_path.parent
        for ref in LINK_REF_RE.findall(text):
            ref = ref.strip()
            if not ref or ref.startswith("#"):
                continue
            if ref.startswith(("http://", "https://", "mailto:", "data:")):
                continue
            path_part = ref.split("#", 1)[0].split("?", 1)[0]
            if not path_part:
                continue
            if any(token in path_part for token in ('"', ": ", "NNN", "...")):
                continue
            candidates = [
                md_dir / path_part,
                REPO_ROOT / path_part,
            ]
            if not any(c.exists() for c in candidates):
                findings.append((_rel(md_path), f"broken link -> {ref}"))
    return findings


def _benchmark_registry_paths() -> tuple[set[str], set[str]]:
    runner_path = REPO_ROOT / "scripts" / "util" / "benchmark_runner.py"
    if not runner_path.is_file():
        return set(), set()
    text = runner_path.read_text(encoding="utf-8")
    sources = set(BENCH_SOURCE_RE.findall(text))
    expecteds = set(BENCH_EXPECTED_RE.findall(text))
    return sources, expecteds


def _manifest_bench_paths() -> set[str]:
    manifest_path = REPO_ROOT / "tests" / "fixtures" / "manifest.txt"
    if not manifest_path.is_file():
        return set()
    _, load_manifest = _import_fixture_helpers()
    paths: set[str] = set()
    for entry in load_manifest(manifest_path):
        if entry["kind"] != "bench":
            continue
        raw = entry["path"]
        normalized = raw.replace("\\", "/")
        if normalized.startswith("../../"):
            normalized = normalized[len("../../") :]
        paths.add(normalized)
    return paths


def _scan_orphan_bench_assets() -> list[tuple[str, str]]:
    benchmarks_root = REPO_ROOT / "benchmarks"
    if not benchmarks_root.is_dir():
        return []

    registered_sources, registered_expected = _benchmark_registry_paths()
    manifest_bench = _manifest_bench_paths()
    registered = registered_sources | manifest_bench

    findings: list[tuple[str, str]] = []
    for path in sorted(benchmarks_root.rglob("*.ark")):
        if _is_excluded(path) or "baselines" in path.parts:
            continue
        rel = _rel(path)
        if rel not in registered:
            findings.append((rel, "not in benchmark_runner.BENCHMARKS or manifest bench:"))

    for path in sorted(benchmarks_root.rglob("*.expected")):
        if _is_excluded(path) or "baselines" in path.parts:
            continue
        rel = _rel(path)
        if rel not in registered_expected and rel.replace(".expected", ".ark") not in registered:
            findings.append((rel, "expected not paired with registered benchmark source"))

    for path in sorted(benchmarks_root.rglob("*.wasm")):
        if _is_excluded(path) or "baselines" in path.parts:
            continue
        rel = _rel(path)
        stem_ark = str(path.with_suffix(".ark").relative_to(REPO_ROOT)).replace("\\", "/")
        if stem_ark not in registered:
            findings.append((rel, "wasm asset without registered benchmark .ark"))

    return findings


def _print_section(title: str, rows: list[tuple[str, ...]]) -> int:
    print(f"=== {title} ({len(rows)}) ===")
    if not rows:
        print("  (none)")
        print()
        return 0
    for row in rows:
        if len(row) == 3:
            path, detail, status = row
            print(f"  {path}")
            print(f"    detail: {detail}")
            print(f"    refs: {status}")
        else:
            path, status = row
            print(f"  {path}")
            print(f"    refs: {status}")
    print()
    return len(rows)


def main() -> int:
    print("=== Orphan / Stale File Inventory (advisory) ===")
    print(f"repo: {REPO_ROOT}")
    print(f"scan roots: {', '.join(SCAN_ROOTS)} + artifact subdirs")
    print(f"large-file threshold: {LARGE_FILE_THRESHOLD // 1024}KB")
    print()

    large_files = _scan_large_files()
    large_rows = [
        (path, f"{size:,} bytes", status) for path, size, status in large_files
    ]

    orphan_fixtures = _scan_orphan_fixtures()
    orphan_expected = _scan_orphan_expected()
    broken_docs = _scan_broken_doc_refs()
    orphan_bench = _scan_orphan_bench_assets()

    total = 0
    total += _print_section("1. Large files", large_rows)
    total += _print_section("2. Orphan fixtures", orphan_fixtures)
    total += _print_section("3. Orphan .expected", orphan_expected)
    total += _print_section("4. Broken doc refs (docs/)", broken_docs)
    total += _print_section("5. Orphan bench assets", orphan_bench)

    print("=== Summary ===")
    print(f"Candidate items: {total}")
    print("Advisory only — exit 0 (review candidates manually).")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
