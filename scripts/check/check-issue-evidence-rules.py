#!/usr/bin/env python3
"""Issue evidence-class audit rules (docs/process/false-done-prevention.md).

Encodes the #678 close-gate rules for ``issues/done/``:

- fixture-evidence: a done issue must cite runnable test evidence
  (a fixture path, manifest class, or a verify command) — not only prose.
- runtime-evidence: issues whose track is runtime-visible must cite an
  execution result (wasmtime run / exit code / PASS), not compile-only
  proof.
- callable-import-evidence: component/WIT import issues must cite an
  invocation (call/invoke/run) result — a guard-only rejection fixture
  does not prove a callable path.
- implementation-evidence: implementation tracks must cite a code change
  (commit id or a non-Markdown source path); a docs-only slice cannot
  close them.

Ratchet: violations already present in ``issues/done/`` live in
``docs/data/issue-evidence-rules-baseline.toml`` and are grandfathered;
a violation NOT in the baseline fails. ``--write-baseline`` regenerates
the baseline from the current tree (review the diff — it records debt,
not endorsement).
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
DONE_DIR = REPO_ROOT / "issues" / "done"
BASELINE = REPO_ROOT / "docs" / "data" / "issue-evidence-rules-baseline.toml"

ISSUE_ID_RE = re.compile(r"^(\d{3})")
FRONTMATTER_RE = re.compile(r"^---\s*$(.*?)^---\s*$", re.MULTILINE | re.DOTALL)

# Evidence citation patterns. Deliberately literal — an issue cites
# evidence by naming the artifact/command, not by adjacency.
FIXTURE_EVIDENCE_RE = re.compile(
    r"tests/fixtures/[\w./-]+"
    r"|\bfixture[s]?\b"
    r"|manifest\.txt"
    r"|\b(?:run|compile|diag|t3-compile|t3-run|component-compile|deny):\s*\S+",
    re.IGNORECASE,
)
VERIFY_EVIDENCE_RE = re.compile(
    r"verify (?:lane|quick|fixtures|full|component-interop)"
    r"|selfhost-fixture-test\.py"
    r"|fixture-parity"
    r"|wasm-tools validate"
    r"|\bwat2wasm\b"
    r"|\bparsed and validated\b",
    re.IGNORECASE,
)
RUN_EVIDENCE_RE = re.compile(
    r"wasmtime run|wasmtime\b"
    r"|\bexecuted\b|\bran\b|\bruns\b"
    r"|exit (?:code|status|status code)?\s*\d+"
    r"|\bexit 0\b"
    r"|\bPASS(?:ED)?\b"
    r"|\brun:\s*\S+"
    r"|stdout|stderr|prints?\b|output",
    re.IGNORECASE,
)
CALLABLE_EVIDENCE_RE = re.compile(
    r"\bcall(?:able|ed|ing|s)?\b"
    r"|\binvok(?:e|ed|es|ing|ation)\b"
    r"|\bimport(?:ed|ing|s)?\b.{0,40}(?:run|call|invoke|execut)"
    r"|host function|host func",
    re.IGNORECASE | re.DOTALL,
)
GUARD_ONLY_RE = re.compile(
    r"reject|guard|unsupported|deny|denied|diagnostic|error[s]?\b",
    re.IGNORECASE,
)
CODE_EVIDENCE_RE = re.compile(
    r"\b[0-9a-f]{7,10}\b"                       # commit id
    r"|src/[\w./-]+\.ark"                        # compiler source
    r"|std/[\w./-]+\.ark"                        # stdlib source
    r"|scripts/[\w./-]+"                         # tooling source
    r"|\.ark\b"                                  # any ark file
    r"|\bcommit\b"
    r"|\.py\b|\.sh\b|\.rs\b|\.toml\b",
    re.IGNORECASE,
)

# Track classification by keyword — frontmatter Track values are
# free-form ("compiler / runtime-perf", "docs, ci"), so match substrings.
DOCS_ONLY_TRACK_RE = re.compile(
    r"docs|documentation|language-design|process|meta|research",
    re.IGNORECASE,
)
RUNTIME_TRACK_RE = re.compile(
    r"component|wasi|wit|runtime|wasm|native|playground|cli|lsp|"
    r"extension|benchmark|selfhost|gc|emit|backend|interop|dap",
    re.IGNORECASE,
)
CALLABLE_IMPORT_TRACK_RE = re.compile(
    r"component|wasi|wit|import|interop",
    re.IGNORECASE,
)
IMPL_TRACK_RE = re.compile(
    r"compiler|codegen|emit|wasm|gc|mir|corehir|parser|typecheck|formatter|"
    r"stdlib-api|stdlib|selfhost|native|backend|opt|lsp|"
    r"monomorph|mono|lowering|implementation",
    re.IGNORECASE,
)

RULES = (
    "fixture-evidence",
    "runtime-evidence",
    "callable-import-evidence",
    "implementation-evidence",
)


def _frontmatter(text: str) -> dict[str, str]:
    m = FRONTMATTER_RE.match(text)
    fields: dict[str, str] = {}
    if not m:
        return fields
    for line in m.group(1).splitlines():
        if ":" in line:
            key, _, val = line.partition(":")
            fields[key.strip().lower()] = val.strip()
    return fields


def _issue_track(text: str, name: str) -> str:
    track = _frontmatter(text).get("track", "")
    if track:
        return track
    # Older issues may lack frontmatter; the summary/body still carries
    # domain words. Use the whole text for classification in that case.
    return name


def violations_for(issue_id: str, path: Path) -> list[str]:
    text = path.read_text(encoding="utf-8")
    track_field = _issue_track(text, path.name)
    classification_text = track_field + "\n" + text[:4000]
    out: list[str] = []

    docs_only = bool(DOCS_ONLY_TRACK_RE.search(track_field)) and not bool(
        IMPL_TRACK_RE.search(track_field)
    )
    runtime_track = bool(RUNTIME_TRACK_RE.search(classification_text))
    callable_track = bool(CALLABLE_IMPORT_TRACK_RE.search(classification_text))
    impl_track = bool(IMPL_TRACK_RE.search(classification_text))

    has_fixture = bool(FIXTURE_EVIDENCE_RE.search(text)) or bool(
        VERIFY_EVIDENCE_RE.search(text)
    )
    has_run = bool(RUN_EVIDENCE_RE.search(text))
    has_callable = bool(CALLABLE_EVIDENCE_RE.search(text))
    has_code = bool(CODE_EVIDENCE_RE.search(text))

    if not docs_only and not has_fixture:
        out.append("fixture-evidence")
    if runtime_track and not has_run:
        out.append("runtime-evidence")
    if callable_track and GUARD_ONLY_RE.search(text) and not has_callable:
        out.append("callable-import-evidence")
    if impl_track and not has_code:
        out.append("implementation-evidence")
    return out


def scan() -> dict[str, list[str]]:
    violations: dict[str, list[str]] = {}
    if not DONE_DIR.is_dir():
        return violations
    for path in sorted(DONE_DIR.glob("*.md")):
        m = ISSUE_ID_RE.match(path.name)
        if not m:
            continue
        v = violations_for(m.group(1), path)
        if v:
            violations[m.group(1)] = v
    return violations


def load_baseline() -> dict[str, list[str]]:
    baseline: dict[str, list[str]] = {}
    if not BASELINE.is_file():
        return baseline
    current_rule = ""
    in_issues = False
    for line in BASELINE.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if in_issues:
            if line.startswith("]"):
                in_issues = False
                continue
            for tok in re.findall(r'"(\d{3}[a-z]?)"', line):
                baseline.setdefault(tok, []).append(current_rule)
            continue
        m = re.match(r"\[(\S+)\]", line)
        if m:
            current_rule = m.group(1)
            continue
        if re.match(r"issues\s*=\s*\[", line) and current_rule:
            in_issues = True
    return baseline


def write_baseline(violations: dict[str, list[str]]) -> None:
    lines = [
        "# Baseline for scripts/check/check-issue-evidence-rules.py (#678).",
        "# Each entry is a PRE-EXISTING violation grandfathered at baseline",
        "# time — regenerate only via --write-baseline and review the diff:",
        "# an entry records debt to revisit, never an endorsement.",
        "",
    ]
    by_rule: dict[str, list[str]] = {rule: [] for rule in RULES}
    for issue_id, rules in sorted(violations.items()):
        for rule in rules:
            if rule in by_rule:
                by_rule[rule].append(issue_id)
    for rule in RULES:
        lines.append(f"[{rule}]")
        ids = sorted(set(by_rule[rule]))
        lines.append("issues = [")
        for issue_id in ids:
            lines.append(f'    "{issue_id}",')
        lines.append("]")
        lines.append("")
    BASELINE.write_text("\n".join(lines), encoding="utf-8")


def main() -> int:
    write = "--write-baseline" in sys.argv
    violations = scan()
    if write:
        write_baseline(violations)
        total = sum(len(v) for v in violations.values())
        print(
            f"wrote {BASELINE}: {len(violations)} issues, "
            f"{total} rule violations"
        )
        return 0

    baseline = load_baseline()
    new_violations: list[str] = []
    for issue_id, rules in sorted(violations.items()):
        known = set(baseline.get(issue_id, []))
        for rule in rules:
            if rule not in known:
                new_violations.append(f"{issue_id}: {rule}")
    stale = [
        f"{issue_id}: {rule}"
        for issue_id, rules in baseline.items()
        for rule in rules
        if rule not in violations.get(issue_id, [])
    ]

    if new_violations:
        print("issue evidence-rule violations not in baseline:")
        for entry in new_violations:
            print(f"  {entry}")
    if stale:
        print("stale baseline entries (issue now complies — drop them):")
        for entry in stale[:20]:
            print(f"  {entry}")
        if len(stale) > 20:
            print(f"  ... and {len(stale) - 20} more")
    if new_violations:
        print(
            "Fix the cited issue's evidence or extend the baseline with "
            "--write-baseline (debt requires an open tracking issue)."
        )
        return 1
    if stale:
        print("advisory: baseline has stale entries; --write-baseline prunes them")
    print(
        f"issue evidence rules: PASS "
        f"({sum(len(v) for v in violations.values())} grandfathered, "
        f"0 new)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
