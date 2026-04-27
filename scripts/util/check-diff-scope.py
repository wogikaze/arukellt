#!/usr/bin/env python3
"""Gate: ensure changed files stay within allowed path prefixes (automation / agent scope).

Used by orchestrators and CI to reject out-of-scope diffs before merge.

Examples:
  python3 scripts/util/check-diff-scope.py \\
    --base origin/master --head HEAD \\
    --primary crates/arukellt/src/main.rs \\
    --allowed crates/arukellt/

  # Staged changes only (pre-commit style):
  python3 scripts/util/check-diff-scope.py --staged \\
    --primary std/ --allowed docs/current-state.md

Forbidden globs use path-prefix semantics (substring path segments), not gitignore globs.
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path


def _repo_root() -> Path:
    current = Path(__file__).resolve().parent
    while True:
        if (current / "Cargo.toml").exists():
            return current
        parent = current.parent
        if parent == current:
            return Path.cwd()
        current = parent


def _norm(p: str) -> str:
    return p.strip().replace("\\", "/").lstrip("./")


def _under_prefix(path: str, prefix: str) -> bool:
    path_n = _norm(path)
    pref = _norm(prefix).rstrip("/")
    if not pref:
        return True
    return path_n == pref or path_n.startswith(pref + "/")


def _matches_any_prefix(path: str, prefixes: list[str]) -> bool:
    return any(_under_prefix(path, p) for p in prefixes if p.strip())


def _git_names_only(repo: Path, args: argparse.Namespace) -> list[str]:
    if args.staged:
        cmd = ["git", "diff", "--cached", "--name-only", "--diff-filter=ACMRT"]
    else:
        cmd = [
            "git",
            "diff",
            "--name-only",
            "--diff-filter=ACMRT",
            f"{args.base}...{args.head}",
        ]
    r = subprocess.run(cmd, cwd=str(repo), capture_output=True, text=True)
    if r.returncode != 0:
        print(r.stderr or r.stdout, file=sys.stderr)
        sys.exit(r.returncode)
    return [ln.strip() for ln in r.stdout.splitlines() if ln.strip()]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--base",
        default="origin/master",
        help="Merge base for range diff (unused with --staged)",
    )
    parser.add_argument("--head", default="HEAD", help="Tip ref for range diff")
    parser.add_argument(
        "--staged",
        action="store_true",
        help="Check index (staged) changes only",
    )
    parser.add_argument(
        "--primary",
        action="append",
        default=[],
        metavar="PATH",
        help="Allowed path (repeatable). File or directory prefix.",
    )
    parser.add_argument(
        "--allowed",
        action="append",
        default=[],
        metavar="PATH",
        help="Extra allowed prefixes (repeatable).",
    )
    parser.add_argument(
        "--forbidden",
        action="append",
        default=[],
        metavar="PATH",
        help="If any changed file is under this prefix, fail (repeatable).",
    )
    parser.add_argument(
        "--allow-empty",
        action="store_true",
        help="Exit 0 when there are no changed files (default: fail)",
    )
    parser.add_argument(
        "--allow-any",
        action="store_true",
        help="Skip allowed-prefix checks; only --forbidden is enforced (audits / repo-wide slices)",
    )
    args = parser.parse_args()

    allowed = [_norm(p) for p in (*args.primary, *args.allowed) if p.strip()]
    forbidden = [_norm(p) for p in args.forbidden if p.strip()]

    if not allowed and not args.allow_any:
        print(
            "ERROR: specify at least one --primary/--allowed, or pass --allow-any",
            file=sys.stderr,
        )
        return 2

    repo = _repo_root()
    changed = _git_names_only(repo, args)

    if not changed:
        if args.allow_empty:
            print("check-diff-scope: no changed files (allow-empty)")
            return 0
        print("ERROR: no changed files in diff; did you commit?", file=sys.stderr)
        return 1

    bad_forbidden: list[str] = []
    for f in changed:
        if _matches_any_prefix(f, forbidden):
            bad_forbidden.append(f)

    if bad_forbidden:
        print("ERROR: changes touch forbidden paths:", file=sys.stderr)
        for f in bad_forbidden:
            print(f"  {f}", file=sys.stderr)
        return 1

    bad_allowed: list[str] = []
    if not args.allow_any:
        for f in changed:
            if not _matches_any_prefix(f, allowed):
                bad_allowed.append(f)

    if bad_allowed:
        print("ERROR: out-of-scope files (not under --primary/--allowed):", file=sys.stderr)
        for f in bad_allowed:
            print(f"  {f}", file=sys.stderr)
        print("\nAllowed prefixes:", file=sys.stderr)
        for p in allowed:
            print(f"  {p}", file=sys.stderr)
        return 1

    print(f"check-diff-scope: OK ({len(changed)} file(s))")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
