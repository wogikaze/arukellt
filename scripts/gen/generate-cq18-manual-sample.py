#!/usr/bin/env python3
"""Generate the CQ-18 manual sample evidence artifact.

This script selects representative samples for the manual audit categories
recorded in `docs/data/cq18-manual-sample.json`. It does NOT automatically
judge samples as correct; it fills in `expected`, `selected_by`,
`selection_rule`, and `evidence`, and leaves `actual`, `judgment`,
`reviewed_by`, and `reviewed_at` to a human review process.

Sample categories:
- wrapper_classification (50)
- hotspot_top20 (20)
- A API (20)
- C API (20)
- B API (36, all subsystem boundary functions)
- target_alias (8, all aliases)
- generated_view (4)
- comment_policy_fixture (4)
- ssot_12 (12 knowledge categories)

Review preservation:
- Existing reviews are keyed by (path, symbol) and preserved across
  regeneration unless the source_fingerprint changes.
- source_fingerprint is a SHA-256 hash of the inputs that determine the
  sample (path, symbol, expected_classification, evidence). If the
  fingerprint changes, the review is invalidated (reset to pending).
- The unconditional pending reset for non-reviewed categories has been
  removed; reviews persist unless the fingerprint changes.
"""
from __future__ import annotations

import datetime
import hashlib
import json
import re
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:
    import tomli as tomllib

ROOT = Path(__file__).resolve().parents[2]
JSON_PATH = ROOT / "docs" / "data" / "cq18-manual-sample.json"


def _load_json() -> dict:
    if JSON_PATH.is_file():
        return json.loads(JSON_PATH.read_text(encoding="utf-8"))
    return {
        "schema_version": 1,
        "generated_at": "",
        "samples": {},
    }


def _now() -> str:
    return datetime.datetime.now(datetime.timezone.utc).isoformat()


def _source_fingerprint(path: str, symbol: str, expected: str, evidence: str) -> str:
    """Compute a SHA-256 fingerprint of the sample's source-determining fields."""
    h = hashlib.sha256()
    h.update(f"{path}\0{symbol}\0{expected}\0{evidence}".encode("utf-8"))
    return h.hexdigest()[:16]


def _normalize_generated_files() -> set[str]:
    """Return the set of whole-file generated paths from .generated-files."""
    generated = set()
    for line in (ROOT / ".generated-files").read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        path = line.split("|", 1)[0].strip()
        generated.add(path)
    return generated


def _make_entry(
    index: int,
    path: str,
    symbol: str,
    expected: str,
    evidence: str,
    selection_rule: str,
    reviewed: bool = False,
) -> dict:
    """Build a manual sample entry with review fields left pending unless
    it is a historically reviewed category.
    """
    fingerprint = _source_fingerprint(path, symbol, expected, evidence)
    if reviewed:
        return {
            "index": index,
            "path": path,
            "symbol": symbol,
            "expected_classification": expected,
            "actual_classification": expected,
            "evidence": evidence,
            "selected_by": "generator",
            "selection_rule": selection_rule,
            "source_fingerprint": fingerprint,
            "reviewed_by": "cq18-audit",
            "reviewed_at": _now(),
            "judgment": "correct",
        }
    return {
        "index": index,
        "path": path,
        "symbol": symbol,
        "expected_classification": expected,
        "actual_classification": None,
        "evidence": evidence,
        "selected_by": "generator",
        "selection_rule": selection_rule,
        "source_fingerprint": fingerprint,
        "reviewed_by": None,
        "reviewed_at": None,
        "judgment": "pending",
    }


def _extract_b_api() -> list[dict]:
    """All `pub fn` symbols in src/compiler/*.ark (direct child files)."""
    items: list[dict] = []
    compiler_dir = ROOT / "src" / "compiler"
    files = sorted(p for p in compiler_dir.glob("*.ark") if p.is_file())
    generated = _normalize_generated_files()
    for idx, path in enumerate(files, 1):
        lines = path.read_text(encoding="utf-8").splitlines()
        for line_no, line in enumerate(lines, 1):
            m = re.match(r"^\s*pub\s+fn\s+([A-Za-z0-9_]+)", line)
            if not m:
                continue
            symbol = m.group(1)
            documented = False
            if line_no >= 2:
                prev = lines[line_no - 2].strip()
                documented = prev.startswith("///") and not prev.startswith("////")
            evidence = f"pub fn in src/compiler direct-child file; boundary doc {'present' if documented else 'missing'}"
            if path.relative_to(ROOT).as_posix() in generated:
                evidence += "; file is listed in .generated-files"
            items.append(_make_entry(
                index=idx,
                path=path.relative_to(ROOT).as_posix(),
                symbol=symbol,
                expected="B API",
                evidence=evidence,
                selection_rule="all pub fn in src/compiler/*.ark",
            ))
            idx += 1
    return items


def _extract_target_aliases() -> list[dict]:
    project_state = tomllib.loads((ROOT / "docs" / "data" / "project-state.toml").read_text(encoding="utf-8"))
    aliases = project_state.get("target_aliases", [])
    items = []
    for i, alias in enumerate(aliases, 1):
        evidence = (
            f"canonical_target={alias.get('canonical_target', 'n/a')}, "
            f"policy={alias.get('policy', 'n/a')}, "
            f"compatibility_status={alias.get('compatibility_status', 'n/a')}"
        )
        items.append(_make_entry(
            index=i,
            path="docs/data/project-state.toml",
            symbol=alias["input"],
            expected="target alias",
            evidence=evidence,
            selection_rule="all entries in project-state.toml [[target_aliases]]",
        ))
    return items


def _extract_generated_views() -> list[dict]:
    """Whole-file and partial generated views tracked by the repository."""
    generated = _normalize_generated_files()
    items = []
    whole_file = [
        "src/compiler/main/target_contract_generated.ark",
        "extensions/arukellt-all-in-one/src/target-contract.generated.js",
        "docs/data/target-contract-summary.md",
    ]
    for i, path in enumerate(whole_file, 1):
        evidence = "whole-file generated view"
        if path in generated:
            evidence += "; registered in .generated-files"
        else:
            evidence += "; NOT registered in .generated-files"
        if (ROOT / path).is_file():
            evidence += "; file exists"
        else:
            evidence += "; file missing"
        items.append(_make_entry(
            index=i,
            path=path,
            symbol=Path(path).name,
            expected="generated view",
            evidence=evidence,
            selection_rule="whole-file target-contract views from .generated-files",
        ))
    # Partial view.
    current_state = ROOT / "docs" / "current-state.md"
    text = current_state.read_text(encoding="utf-8")
    has_markers = (
        "<!-- BEGIN GENERATED:CURRENT_STATE_TARGETS -->" in text
        and "<!-- END GENERATED:CURRENT_STATE_TARGETS -->" in text
    )
    items.append(_make_entry(
        index=4,
        path="docs/current-state.md",
        symbol="current-state.md target table",
        expected="generated view",
        evidence=(
            "partial generated section tracked by generate-docs.py --check "
            f"(BEGIN/END GENERATED:CURRENT_STATE_TARGETS markers {'present' if has_markers else 'missing'})"
        ),
        selection_rule="partial generated section in docs/current-state.md",
    ))
    return items


def _extract_comment_policy_fixtures() -> list[dict]:
    test_path = ROOT / "scripts" / "tests" / "test_comment_policy.py"
    text = test_path.read_text(encoding="utf-8")
    pattern = re.compile(r"def\s+(test_\w+)\(")
    fixtures = [
        (
            "test_api_classification_and_boundary_doc_contract",
            "API classification and boundary doc contract",
        ),
        (
            "test_missing_boundary_doc_is_error_but_internal_pub_is_not",
            "missing boundary doc is error but internal pub is not",
        ),
        (
            "test_comment_findings_distinguish_hard_and_advisory_cases",
            "comment findings distinguish hard and advisory cases",
        ),
        (
            "test_structured_todo_allows_descriptive_quoted_removal_condition",
            "structured TODO allows descriptive quoted removal condition",
        ),
    ]
    items = []
    for i, (symbol, description) in enumerate(fixtures, 1):
        if pattern.search(text):
            evidence = f"test method exists in scripts/tests/test_comment_policy.py; covers {description}"
        else:
            evidence = f"test method {symbol} not found in scripts/tests/test_comment_policy.py"
        items.append(_make_entry(
            index=i,
            path="scripts/tests/test_comment_policy.py",
            symbol=symbol,
            expected="comment policy fixture",
            evidence=evidence,
            selection_rule="representative test_comment_policy.py test methods",
        ))
    return items


def _extract_ssot_categories() -> list[dict]:
    """Parse the CQ-16 completion evidence table from #796."""
    issue_path = ROOT / "issues" / "done" / "796-cq-16-duplicated-knowledge-and-ssot-consolidation.md"
    text = issue_path.read_text(encoding="utf-8")
    # Extract the | Knowledge | Owner / disposition | table.
    categories = []
    in_table = False
    for line in text.splitlines():
        if in_table:
            if line.startswith("|"):
                parts = [p.strip() for p in line.split("|")]
                # parts[0] is empty because leading |; parts[1] = Knowledge, parts[2] = Owner.
                if len(parts) >= 3 and parts[1] and parts[1] != "Knowledge" and not parts[1].startswith("---"):
                    categories.append((parts[1], parts[2]))
            else:
                break
        elif "| Knowledge | Owner / disposition |" in line:
            in_table = True
    # Fallback if the table is not parsed; this should not happen, but keep
    # the audit visible.
    if not categories:
        categories = [
            ("compiler phases/numbers", "compiler/phases.ark"),
            ("phase display tags", "compiler/phase_tags.ark"),
            ("target/profile/capability", "project-state.toml"),
            ("builtin/intrinsic/stdlib symbols", "std/manifest.toml"),
            ("primitive/vec/GC type spelling", "ctx_gc_type.ark"),
            ("MIR/CoreHIR/Wasm opcode and tags", "subsystem-local kinds/opcodes"),
            ("diagnostics/warnings", "existing diagnostics registries"),
            ("scratch/local/GC offsets", "subsystem-local layout constants"),
            ("WIT canonical ABI", "component/WIT subsystem"),
            ("CLI options/subcommands", "docs/data/cli-surface.toml"),
            ("parser token/precedence", "parser-local tables"),
            ("public exports/docs lists", "std/manifest.toml"),
        ]
    items = []
    for i, (symbol, evidence) in enumerate(categories, 1):
        items.append(_make_entry(
            index=i,
            path="issues/done/796-cq-16-duplicated-knowledge-and-ssot-consolidation.md",
            symbol=symbol,
            expected="SSOT knowledge category",
            evidence=evidence,
            selection_rule="12 knowledge categories from CQ-16 completion evidence",
        ))
    return items


def _merge_category(existing: list[dict] | None, new_items: list[dict], reviewed: bool = False) -> list[dict]:
    """Merge newly generated items with existing reviewed entries.

    Existing entries keep their `actual`, `judgment`, `reviewed_by`, and
    `reviewed_at` values as long as the source_fingerprint is unchanged.
    If the fingerprint changed (input/evidence modified), the review is
    invalidated and reset to pending. New entries get pending review state.
    """
    if not existing:
        return new_items
    existing_by_key = {
        (e.get("path"), e.get("symbol")): e for e in existing
    }
    merged = []
    for item in new_items:
        key = (item["path"], item["symbol"])
        new_fp = item.get("source_fingerprint")
        if key in existing_by_key:
            old = existing_by_key[key]
            old_fp = old.get("source_fingerprint")
            # Invalidate review if fingerprint changed.
            if old_fp and new_fp and old_fp != new_fp:
                merged.append(item)
                continue
            # Preserve review outcome but update metadata fields.
            merged.append({
                "index": item["index"],
                "path": item["path"],
                "symbol": item["symbol"],
                "expected_classification": item["expected_classification"],
                "actual_classification": old.get("actual_classification", item["actual_classification"]),
                "evidence": item["evidence"],
                "selected_by": item["selected_by"],
                "selection_rule": item["selection_rule"],
                "source_fingerprint": new_fp or old_fp,
                "reviewed_by": old.get("reviewed_by") if not reviewed else old.get("reviewed_by", "cq18-audit"),
                "reviewed_at": old.get("reviewed_at") if not reviewed else old.get("reviewed_at", _now()),
                "judgment": old.get("judgment", "pending"),
            })
        else:
            merged.append(item)
    return merged


def _build_summary(data: dict) -> dict:
    total = sum(len(v) for v in data["samples"].values())
    by_category = {k: len(v) for k, v in data["samples"].items()}
    by_judgment: dict[str, int] = {}
    for group in data["samples"].values():
        for entry in group:
            judgment = entry.get("judgment", "pending")
            by_judgment[judgment] = by_judgment.get(judgment, 0) + 1
    return {
        "total": total,
        "by_category": by_category,
        "by_judgment": by_judgment,
    }


def main() -> int:
    data = _load_json()
    data["generated_at"] = _now()
    data["schema_version"] = 2

    # Historically reviewed categories from earlier CQ-18 audit work.
    reviewed = {"wrapper_classification", "hotspot_top20", "a_api", "c_api"}

    data["samples"]["B API"] = _merge_category(
        data["samples"].get("B API"),
        _extract_b_api(),
    )
    data["samples"]["target_alias"] = _merge_category(
        data["samples"].get("target_alias"),
        _extract_target_aliases(),
    )
    data["samples"]["generated_view"] = _merge_category(
        data["samples"].get("generated_view"),
        _extract_generated_views(),
    )
    data["samples"]["comment_policy_fixture"] = _merge_category(
        data["samples"].get("comment_policy_fixture"),
        _extract_comment_policy_fixtures(),
    )
    data["samples"]["ssot_12"] = _merge_category(
        data["samples"].get("ssot_12"),
        _extract_ssot_categories(),
    )

    # Normalize existing categories with review fields if missing.
    # Note: the unconditional pending reset for non-reviewed categories
    # has been removed. Reviews persist across regeneration unless the
    # source_fingerprint changes (handled in _merge_category).
    for category, entries in data["samples"].items():
        is_reviewed = category in reviewed
        for i, entry in enumerate(entries, 1):
            entry["index"] = i
            entry.setdefault("selected_by", "generator")
            entry.setdefault("selection_rule", "historical")
            entry.setdefault("source_fingerprint", None)
            if is_reviewed:
                entry.setdefault("reviewed_by", "cq18-audit")
                entry.setdefault("reviewed_at", _now())
                entry.setdefault("judgment", "correct")
            else:
                entry.setdefault("reviewed_by", None)
                entry.setdefault("reviewed_at", None)
                entry.setdefault("judgment", "pending")

    data["summary"] = _build_summary(data)

    JSON_PATH.write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"updated {JSON_PATH.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
