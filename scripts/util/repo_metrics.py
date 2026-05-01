#!/usr/bin/env python3
"""Repository metrics counter.

Measures lines, characters, bytes by file extension, source area, and content
kind across a repository. Supports git-tracked files or filesystem walk,
with special classification for Rust (.rs), Nepl (.nepl), and Markdown files.

Usage:
    python3 scripts/repo_metrics.py [options]

Options:
    --root <path>         Repo root or subdir (default: .)
    --mode <auto|git|walk>  File discovery mode (default: auto)
    --suffix-mode <all|last> Extension grouping mode (default: all)
    --max-bytes <n>       Skip text counting above this size (default: 5000000, 0 disables)
    --binary <skip|bytes>  Skip binaries or count only file size (default: skip)
    --csv <path>          Write flattened CSV
    --json <path>         Write structured JSON
    -h, --help            Show this help
"""

from __future__ import annotations

import argparse
import csv
import json
import os
import re
import stat
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import IO

IS_WIN = sys.platform == "win32"

# ── constants ──────────────────────────────────────────────────────────────────

TOP_LEVEL_DOC_TEST_DIRS = frozenset({"tests", "tutorials", "doc", "examples"})

SOURCE_EXTS = frozenset({
    ".c", ".cpp", ".css", ".h", ".hpp", ".html", ".java", ".js", ".jsx",
    ".mjs", ".mts", ".nepl", ".py", ".rb", ".rs", ".sh", ".sql", ".ts",
    ".tsx", ".wat", ".wast", ".wasm", ".yaml", ".yml",
})

MARKDOWN_SUFFIXES = frozenset({".md", ".n.md"})

CONTENT_KINDS = ("blank", "source", "doc_comment", "document", "test", "comment", "other")

# Doctest patterns
DOCTEST_META_RE = re.compile(
    r"^\s*(stdin|argv|stdout|stderr|ret|diag_code|diag_codes|diag_span|diag_spans)\s*:\s*(.*?)\s*$"
)
DOCTEST_RE = re.compile(r"^\s*neplg2:test(?:\[[^\]]+\])?\s*$")
DOCTEST_FENCE_OPEN_RE = re.compile(r"^\s*```neplg2\s*$")
DOCTEST_FENCE_CLOSE_RE = re.compile(r"^\s*```\s*$")

# Nepl doc comment
NEPL_DOC_RE = re.compile(r"^\s*\/\/:(\|)?\s?(.*)$")

# Rust patterns
RUST_DOC_RE = re.compile(r"^\s*(///|//!)")
RUST_COMMENT_RE = re.compile(r"^\s*//")
RUST_CFG_TEST_RE = re.compile(r"^\s*#\[\s*cfg\s*\(\s*test\s*\)\s*\]")
RUST_TEST_ATTR_RE = re.compile(r"^\s*#\[(?:test|tokio::test|wasm_bindgen_test)\b")
RUST_FN_RE = re.compile(r"^\s*(?:pub\s+)?(?:async\s+)?fn\b")


# ── data types ─────────────────────────────────────────────────────────────────


@dataclass
class FileStats:
    lines: int = 0
    chars: int = 0
    bytes: int = 0
    blank: int = 0
    source: int = 0
    doc_comment: int = 0
    document: int = 0
    test: int = 0
    comment: int = 0
    other: int = 0
    test_cases: int = 0
    kind_chars: dict[str, int] = field(default_factory=dict)
    kind_bytes: dict[str, int] = field(default_factory=dict)

    def __post_init__(self):
        if not self.kind_chars:
            self.kind_chars = {k: 0 for k in CONTENT_KINDS}
        if not self.kind_bytes:
            self.kind_bytes = {k: 0 for k in CONTENT_KINDS}


@dataclass
class BucketStats:
    files: int = 0
    lines: int = 0
    chars: int = 0
    bytes: int = 0
    blank: int = 0
    source: int = 0
    doc_comment: int = 0
    document: int = 0
    test: int = 0
    comment: int = 0
    other: int = 0
    test_cases: int = 0


@dataclass
class BucketFull(BucketStats):
    kind_chars: dict[str, int] = field(default_factory=lambda: {k: 0 for k in CONTENT_KINDS})
    kind_bytes: dict[str, int] = field(default_factory=lambda: {k: 0 for k in CONTENT_KINDS})


@dataclass
class SimpleStats:
    files: int = 0
    lines: int = 0
    chars: int = 0
    bytes: int = 0
    test_cases: int = 0


# ── helpers ────────────────────────────────────────────────────────────────────


class TextLine:
    __slots__ = ("text", "raw_bytes")

    def __init__(self, text: str, raw_bytes: int):
        self.text = text
        self.raw_bytes = raw_bytes


def _fmt(n: int) -> str:
    return f"{n:,}"


def _csv_esc(v: str) -> str:
    if "," not in v and '"' not in v and "\n" not in v:
        return v
    return f'"{v.replace(chr(34), chr(34) + chr(34))}"'


def _ext_key(rel_path: str, suffix_mode: str) -> str:
    basename = rel_path.rsplit("/", 1)[-1] or rel_path
    if suffix_mode == "all":
        idx = basename.find(".")
        return basename[idx:].lower() if idx >= 0 else "(no_ext)"
    ext = Path(basename).suffix.lower()
    return ext or "(no_ext)"


def _classify_area(rel_path: str) -> str:
    parts = rel_path.split("/")
    if not parts:
        return "other"
    if parts[0] in TOP_LEVEL_DOC_TEST_DIRS:
        return "top_level_docs_tests"
    if parts[0] == "stdlib" or "src" in parts:
        return "source_tree"
    return "other"


def _is_test_path(rel_path: str) -> bool:
    return "tests" in rel_path.split("/")


# ── git / walk ─────────────────────────────────────────────────────────────────


def _is_git_repo(path: str | Path) -> bool:
    try:
        subprocess.run(
            ["git", "rev-parse", "--is-inside-work-tree"],
            cwd=str(path),
            capture_output=True,
            check=True,
        )
        return True
    except subprocess.CalledProcessError:
        return False


def _git_root(path: str | Path) -> str:
    result = subprocess.run(
        ["git", "rev-parse", "--show-toplevel"],
        cwd=str(path),
        capture_output=True,
        check=True,
        text=True,
    )
    return result.stdout.strip()


def _list_git_files(root: str | Path) -> list[str]:
    result = subprocess.run(
        ["git", "ls-files", "-z", "--cached", "--others", "--exclude-standard"],
        cwd=str(root),
        capture_output=True,
        check=True,
    )
    raw = result.stdout.decode("utf-8")
    return [v.strip() for v in raw.split("\0") if v.strip()]


def _list_files_walk(root: str | Path) -> list[str]:
    out: list[str] = []
    root_path = Path(root)
    for abs_path in sorted(root_path.rglob("*")):
        if abs_path.is_dir():
            continue
        rel = abs_path.relative_to(root_path).as_posix()
        out.append(rel)
    out.sort()
    return out


# ── binary detection ───────────────────────────────────────────────────────────


def _is_probably_binary(path: str | Path, sample_size: int = 8192) -> bool:
    try:
        with open(path, "rb") as fh:
            head = fh.read(sample_size)
            return b"\0" in head
    except OSError:
        return True


# ── text line reading ──────────────────────────────────────────────────────────


def _read_text_lines(path: str | Path, max_bytes: int | None) -> list[TextLine]:
    raw = Path(path).read_bytes()
    if max_bytes is not None and max_bytes > 0 and len(raw) > max_bytes:
        raise ValueError(f"file too large ({len(raw)} bytes) > max_bytes")
    text = raw.decode("utf-8")
    # Split into lines preserving line endings
    raw_bin = raw.decode("latin-1")
    raw_lines = re.findall(r"[^\r\n]*(?:\r\n|\r|\n|$)", raw_bin)
    text_lines = re.findall(r"[^\r\n]*(?:\r\n|\r|\n|$)", text)
    count = max(len(raw_lines), len(text_lines))
    out: list[TextLine] = []
    for i in range(count):
        rl = raw_lines[i] if i < len(raw_lines) else ""
        tl = text_lines[i] if i < len(text_lines) else ""
        if i == count - 1 and rl == "" and tl == "":
            continue
        out.append(TextLine(tl, len(rl.encode("latin-1"))))
    return out


# ── add line to stats ─────────────────────────────────────────────────────────


def _add_line(stats: FileStats, kind: str, line: TextLine) -> None:
    stats.lines += 1
    stats.chars += len(line.text)
    stats.bytes += line.raw_bytes
    if line.text.strip() == "":
        stats.blank += 1
        stats.kind_chars["blank"] += len(line.text)
        stats.kind_bytes["blank"] += line.raw_bytes
        return
    setattr(stats, kind, getattr(stats, kind) + 1)
    stats.kind_chars[kind] += len(line.text)
    stats.kind_bytes[kind] += line.raw_bytes


# ── classifiers ────────────────────────────────────────────────────────────────


def _classify_markdown(lines: list[TextLine]) -> FileStats:
    stats = FileStats()
    state: str = "document"  # document | await_fence | in_fence

    for line in lines:
        stripped = line.text.rstrip("\r\n")
        if state == "document":
            if DOCTEST_RE.search(stripped):
                _add_line(stats, "test", line)
                stats.test_cases += 1
                state = "await_fence"
            else:
                _add_line(stats, "document", line)
            continue

        if state == "await_fence":
            if DOCTEST_META_RE.match(stripped):
                _add_line(stats, "test", line)
            elif DOCTEST_FENCE_OPEN_RE.match(stripped):
                _add_line(stats, "test", line)
                state = "in_fence"
            else:
                _add_line(stats, "document", line)
                state = "document"
            continue

        # state == "in_fence"
        _add_line(stats, "test", line)
        if DOCTEST_FENCE_CLOSE_RE.match(stripped):
            state = "document"

    return stats


def _classify_nepl(rel_path: str, lines: list[TextLine]) -> FileStats:
    stats = FileStats()
    test_file = _is_test_path(rel_path)
    state: str = "document_comment"

    for line in lines:
        stripped = line.text.rstrip("\r\n")
        m = NEPL_DOC_RE.match(stripped)
        if m:
            doc_text = f"{'|' if m.group(1) else ''}{m.group(2) or ''}"
            if state == "document_comment":
                if DOCTEST_RE.search(doc_text):
                    _add_line(stats, "test", line)
                    stats.test_cases += 1
                    state = "await_fence"
                else:
                    _add_line(stats, "doc_comment", line)
            elif state == "await_fence":
                if DOCTEST_META_RE.match(doc_text):
                    _add_line(stats, "test", line)
                elif DOCTEST_FENCE_OPEN_RE.match(doc_text):
                    _add_line(stats, "test", line)
                    state = "in_fence"
                else:
                    _add_line(stats, "doc_comment", line)
                    state = "document_comment"
            else:
                _add_line(stats, "test", line)
                if DOCTEST_FENCE_CLOSE_RE.match(doc_text):
                    state = "document_comment"
            continue

        if line.text.strip() == "":
            _add_line(stats, "other", line)
        elif stripped.strip().startswith("//"):
            _add_line(stats, "comment", line)
        elif test_file:
            _add_line(stats, "test", line)
        else:
            _add_line(stats, "source", line)

    return stats


def _classify_rust(rel_path: str, lines: list[TextLine]) -> FileStats:
    stats = FileStats()
    test_file = _is_test_path(rel_path)
    brace_depth = 0
    test_region_ends: list[int] = []
    pending_cfg_test = False
    pending_test_attr = False

    for line in lines:
        stripped = line.text.rstrip("\r\n")
        logical = stripped.strip()
        in_test_region = test_file or len(test_region_ends) > 0
        is_cfg_test = bool(RUST_CFG_TEST_RE.match(stripped))
        is_test_attr = bool(RUST_TEST_ATTR_RE.match(stripped))
        is_doc = bool(RUST_DOC_RE.match(stripped))

        if logical == "":
            _add_line(stats, "other", line)
        elif is_cfg_test or is_test_attr:
            _add_line(stats, "test", line)
            if is_cfg_test:
                pending_cfg_test = True
            if is_test_attr:
                pending_test_attr = True
                stats.test_cases += 1
        elif is_doc:
            _add_line(stats, "doc_comment", line)
        elif pending_cfg_test or pending_test_attr or in_test_region:
            _add_line(stats, "test", line)
        elif RUST_COMMENT_RE.match(stripped):
            _add_line(stats, "comment", line)
        else:
            _add_line(stats, "source", line)

        depth_before = brace_depth
        opens = stripped.count("{")
        closes = stripped.count("}")

        # Track cfg(test) blocks
        if pending_cfg_test and logical != "" and not is_cfg_test:
            if "{" in stripped:
                test_region_ends.append(depth_before)
                pending_cfg_test = False
            elif stripped.rstrip().endswith(";"):
                pending_cfg_test = False

        # Track #[test] functions
        if pending_test_attr and logical != "" and not is_test_attr:
            if RUST_FN_RE.match(stripped) and "{" in stripped:
                test_region_ends.append(depth_before)
                pending_test_attr = False
            elif not stripped.startswith("#[") and "{" in stripped:
                test_region_ends.append(depth_before)
                pending_test_attr = False
            elif stripped.rstrip().endswith(";"):
                pending_test_attr = False

        brace_depth += opens - closes
        while test_region_ends and brace_depth <= test_region_ends[-1]:
            test_region_ends.pop()

    return stats


def _classify_generic(rel_path: str, lines: list[TextLine]) -> FileStats:
    stats = FileStats()
    ext_key = _ext_key(rel_path, "all")
    test_file = _is_test_path(rel_path)
    is_markdown = ext_key in MARKDOWN_SUFFIXES or Path(rel_path).suffix.lower() == ".md"
    is_source = Path(rel_path).suffix.lower() in SOURCE_EXTS

    for line in lines:
        if line.text.strip() == "":
            _add_line(stats, "other", line)
        elif is_markdown:
            _add_line(stats, "document", line)
        elif test_file:
            _add_line(stats, "test", line)
        elif is_source:
            _add_line(stats, "source", line)
        else:
            _add_line(stats, "other", line)

    return stats


# ── measure one file ───────────────────────────────────────────────────────────


def _measure_file(rel_path: str, abs_path: str, max_bytes: int | None) -> FileStats:
    lines = _read_text_lines(abs_path, max_bytes)
    ext_key = _ext_key(rel_path, "all")
    suffix = Path(rel_path).suffix.lower()

    if ext_key in MARKDOWN_SUFFIXES or suffix == ".md":
        return _classify_markdown(lines)
    if suffix == ".nepl":
        return _classify_nepl(rel_path, lines)
    if suffix == ".rs":
        return _classify_rust(rel_path, lines)
    return _classify_generic(rel_path, lines)


# ── bucket accumulation ────────────────────────────────────────────────────────


def _accum_bucket(dest: BucketStats, src: FileStats) -> None:
    dest.lines += src.lines
    dest.chars += src.chars
    dest.bytes += src.bytes
    dest.blank += src.blank
    dest.source += src.source
    dest.doc_comment += src.doc_comment
    dest.document += src.document
    dest.test += src.test
    dest.comment += src.comment
    dest.other += src.other
    dest.test_cases += src.test_cases


def _accum_simple(dest: SimpleStats, files: int = 0, lines: int = 0, chars: int = 0, bytes_: int = 0, test_cases: int = 0) -> None:
    dest.files += files
    dest.lines += lines
    dest.chars += chars
    dest.bytes += bytes_
    dest.test_cases += test_cases


# ── sorting ────────────────────────────────────────────────────────────────────


def _sort_buckets(entries: list[tuple[str, BucketStats]]) -> list[tuple[str, BucketStats]]:
    return sorted(
        entries,
        key=lambda x: (
            -x[1].bytes,
            -x[1].lines,
            -x[1].files,
            x[0],
        ),
    )


# ── output ─────────────────────────────────────────────────────────────────────


def _calc_widths(headers: list[str], data: list[list[str]]) -> list[int]:
    widths = [len(h) for h in headers]
    for row in data:
        for i, cell in enumerate(row):
            widths[i] = max(widths[i], len(cell))
    return widths


def _fmt_row(row: list[str], widths: list[int]) -> str:
    parts: list[str] = []
    for i, cell in enumerate(row):
        if i == 0:
            parts.append(cell.ljust(widths[i]))
        else:
            parts.append(cell.rjust(widths[i]))
    return "  ".join(parts)


def _print_bucket_table(title: str, key_name: str, stats: dict[str, BucketStats]) -> None:
    rows = _sort_buckets(list(stats.items()))
    headers = [
        key_name, "files", "lines", "chars", "bytes", "blank", "source",
        "doc_comment", "document", "test", "comment", "other", "test_cases",
    ]
    data: list[list[str]] = []
    for key, s in rows:
        data.append([
            key,
            _fmt(s.files),
            _fmt(s.lines),
            _fmt(s.chars),
            _fmt(s.bytes),
            _fmt(s.blank),
            _fmt(s.source),
            _fmt(s.doc_comment),
            _fmt(s.document),
            _fmt(s.test),
            _fmt(s.comment),
            _fmt(s.other),
            _fmt(s.test_cases),
        ])

    widths = _calc_widths(headers, data)
    print(title)
    print(_fmt_row(headers, widths))
    print(_fmt_row(["-" * len(h) for h in headers], widths))
    for row in data:
        print(_fmt_row(row, widths))

    total = BucketStats()
    for s in stats.values():
        total.files += s.files
        total.lines += s.lines
        total.chars += s.chars
        total.bytes += s.bytes
        total.blank += s.blank
        total.source += s.source
        total.doc_comment += s.doc_comment
        total.document += s.document
        total.test += s.test
        total.comment += s.comment
        total.other += s.other
        total.test_cases += s.test_cases
    print()
    print(_fmt_row([
        "TOTAL",
        _fmt(total.files),
        _fmt(total.lines),
        _fmt(total.chars),
        _fmt(total.bytes),
        _fmt(total.blank),
        _fmt(total.source),
        _fmt(total.doc_comment),
        _fmt(total.document),
        _fmt(total.test),
        _fmt(total.comment),
        _fmt(total.other),
        _fmt(total.test_cases),
    ], widths))


def _print_simple_table(title: str, key_name: str, stats: dict[str, SimpleStats]) -> None:
    rows = sorted(
        stats.items(),
        key=lambda x: (-x[1].bytes, -x[1].lines, -x[1].files, x[0]),
    )
    headers = [key_name, "files", "lines", "chars", "bytes", "test_cases"]
    data: list[list[str]] = []
    for key, s in rows:
        data.append([key, _fmt(s.files), _fmt(s.lines), _fmt(s.chars), _fmt(s.bytes), _fmt(s.test_cases)])

    widths = _calc_widths(headers, data)
    print(title)
    print(_fmt_row(headers, widths))
    print(_fmt_row(["-" * len(h) for h in headers], widths))
    for row in data:
        print(_fmt_row(row, widths))

    total = SimpleStats()
    for s in stats.values():
        _accum_simple(total, s.files, s.lines, s.chars, s.bytes, s.test_cases)
    print()
    print(_fmt_row(["TOTAL", _fmt(total.files), _fmt(total.lines), _fmt(total.chars), _fmt(total.bytes), _fmt(total.test_cases)], widths))


# ── CSV / JSON ─────────────────────────────────────────────────────────────────


def _write_csv(
    path: str,
    ext_stats: dict[str, BucketStats],
    area_stats: dict[str, BucketStats],
    kind_stats: dict[str, SimpleStats],
) -> None:
    with open(path, "w", newline="") as fh:
        writer = csv.writer(fh)
        writer.writerow([
            "section", "name", "files", "lines", "chars", "bytes",
            "blank", "source", "doc_comment", "document", "test", "comment", "other", "test_cases",
        ])
        for section, table in [("extension", ext_stats), ("area", area_stats)]:
            for name in sorted(table):
                s = table[name]
                writer.writerow([
                    section, name, s.files, s.lines, s.chars, s.bytes,
                    s.blank, s.source, s.doc_comment, s.document,
                    s.test, s.comment, s.other, s.test_cases,
                ])
        for name in sorted(kind_stats):
            s = kind_stats[name]
            writer.writerow(["content_kind", name, s.files, s.lines, s.chars, s.bytes, "", "", "", "", "", "", "", s.test_cases])


def _write_json(
    path: str,
    ext_stats: dict[str, BucketStats],
    area_stats: dict[str, BucketStats],
    kind_stats: dict[str, SimpleStats],
    skipped: list[dict[str, str]],
) -> None:
    def bucket_to_dict(s: BucketStats) -> dict:
        return {
            "files": s.files, "lines": s.lines, "chars": s.chars, "bytes": s.bytes,
            "blank": s.blank, "source": s.source, "doc_comment": s.doc_comment,
            "document": s.document, "test": s.test, "comment": s.comment,
            "other": s.other, "testCases": s.test_cases,
        }

    def simple_to_dict(s: SimpleStats) -> dict:
        return {"files": s.files, "lines": s.lines, "chars": s.chars, "bytes": s.bytes, "testCases": s.test_cases}

    payload = {
        "byExtension": [{"name": name, **bucket_to_dict(s)} for name, s in sorted(ext_stats.items())],
        "byArea": [{"name": name, **bucket_to_dict(s)} for name, s in sorted(area_stats.items())],
        "byContentKind": [{"name": name, **simple_to_dict(s)} for name, s in sorted(kind_stats.items())],
        "skipped": skipped,
    }
    with open(path, "w") as fh:
        json.dump(payload, fh, indent=2)
        fh.write("\n")


# ── main ───────────────────────────────────────────────────────────────────────


def _parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Repository metrics counter.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__,
    )
    parser.add_argument("--root", default=".", help="Repo root or subdir (default: .)")
    parser.add_argument(
        "--mode", choices=["auto", "git", "walk"], default="auto",
        help="File discovery mode (default: auto)",
    )
    parser.add_argument(
        "--suffix-mode", choices=["all", "last"], default="all",
        help="Extension grouping mode (default: all)",
    )
    parser.add_argument(
        "--max-bytes", type=int, default=5_000_000,
        help="Skip text counting above this size (default: 5000000, 0 disables)",
    )
    parser.add_argument(
        "--binary", choices=["skip", "bytes"], default="skip",
        help="Skip binaries or count only file size (default: skip)",
    )
    parser.add_argument("--csv", help="Write flattened CSV")
    parser.add_argument("--json", help="Write structured JSON")
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = _parse_args(argv)
    root = Path(args.root).resolve()
    use_git = False

    if args.mode in ("auto", "git"):
        use_git = _is_git_repo(root)
        if args.mode == "git" and not use_git:
            print("ERROR: --mode git but not inside a Git repository.", file=sys.stderr)
            return 2

    if use_git:
        root = Path(_git_root(root))
        rel_paths = _list_git_files(root)
    else:
        rel_paths = _list_files_walk(root)

    ext_stats: dict[str, BucketStats] = {}
    area_stats: dict[str, BucketStats] = {}
    kind_stats: dict[str, SimpleStats] = {}
    skipped: list[dict[str, str]] = []
    max_bytes = None if args.max_bytes == 0 else args.max_bytes

    for rel_path in rel_paths:
        abs_path = root / rel_path
        try:
            st = abs_path.stat()
        except OSError:
            skipped.append({"path": rel_path, "reason": "unreadable"})
            continue
        if not stat.S_ISREG(st.st_mode):
            continue

        ext = _ext_key(rel_path, args.suffix_mode)
        area = _classify_area(rel_path)

        if _is_probably_binary(str(abs_path)):
            if args.binary == "skip":
                skipped.append({"path": rel_path, "reason": "binary"})
                continue
            if ext not in ext_stats:
                ext_stats[ext] = BucketStats()
            if area not in area_stats:
                area_stats[area] = BucketStats()
            ext_stats[ext].files += 1
            ext_stats[ext].bytes += st.st_size
            area_stats[area].files += 1
            area_stats[area].bytes += st.st_size
            continue

        try:
            measured = _measure_file(rel_path, str(abs_path), max_bytes)
        except ValueError as e:
            skipped.append({"path": rel_path, "reason": "too_large"})
            continue
        except OSError:
            skipped.append({"path": rel_path, "reason": "unreadable"})
            continue

        if ext not in ext_stats:
            ext_stats[ext] = BucketStats()
        if area not in area_stats:
            area_stats[area] = BucketStats()
        ext_stats[ext].files += 1
        area_stats[area].files += 1
        _accum_bucket(ext_stats[ext], measured)
        _accum_bucket(area_stats[area], measured)

        for kind in CONTENT_KINDS:
            lines = getattr(measured, kind)
            if lines <= 0:
                continue
            if kind not in kind_stats:
                kind_stats[kind] = SimpleStats()
            bucket = kind_stats[kind]
            bucket.files += 1
            bucket.lines += lines
            bucket.chars += measured.kind_chars[kind]
            bucket.bytes += measured.kind_bytes[kind]
            if kind == "test":
                bucket.test_cases += measured.test_cases

    print()
    _print_bucket_table("By Extension", "ext", ext_stats)
    print()
    _print_bucket_table("By Area", "area", area_stats)
    print()
    _print_simple_table("By Content Kind", "kind", kind_stats)

    if skipped:
        print()
        print(f"Skipped files: {len(skipped)} (showing up to 20)")
        for item in skipped[:20]:
            print(f"  - {item['path']} [{item['reason']}]")
        if len(skipped) > 20:
            print("  ...")

    if args.csv:
        _write_csv(args.csv, ext_stats, area_stats, kind_stats)
    if args.json:
        _write_json(args.json, ext_stats, area_stats, kind_stats, skipped)

    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
