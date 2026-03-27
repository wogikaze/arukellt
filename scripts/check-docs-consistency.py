#!/usr/bin/env python3
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DOCS = ROOT / "docs"
MANIFEST = ROOT / "tests" / "fixtures" / "manifest.txt"


def manifest_count() -> int:
    return sum(
        1
        for line in MANIFEST.read_text().splitlines()
        if line.strip() and not line.strip().startswith("#")
    )


def must_contain(path: Path, needle: str) -> list[str]:
    text = path.read_text()
    return [] if needle in text else [f"{path.relative_to(ROOT)} missing: {needle}"]


def must_match(path: Path, pattern: str, expected: str) -> list[str]:
    text = path.read_text()
    match = re.search(pattern, text)
    if not match:
        return [f"{path.relative_to(ROOT)} missing pattern: {pattern}"]
    actual = match.group(1)
    if actual != expected:
        return [f"{path.relative_to(ROOT)} expected {expected!r}, found {actual!r}"]
    return []


def main() -> int:
    errors: list[str] = []
    count = manifest_count()

    current_state = DOCS / "current-state.md"
    pipeline = DOCS / "compiler" / "pipeline.md"
    contributing = DOCS / "contributing.md"
    wasm_features = DOCS / "platform" / "wasm-features.md"
    migration = DOCS / "migration" / "t1-to-t3.md"
    diagnostics = DOCS / "compiler" / "diagnostics.md"
    policy = DOCS / "process" / "policy.md"

    errors += must_contain(current_state, f"Fixture manifest: {count} entries")
    errors += must_contain(current_state, f"Fixture harness: {count} passed")
    errors += must_contain(pipeline, f"fixture harness は manifest-driven で {count} entries")
    errors += must_contain(contributing, f"All fixture tests ({count} pass, 0 fail)")

    errors += must_contain(current_state, "Wasm validation is a hard error (W0004)")
    errors += must_contain(diagnostics, "W0004")
    errors += must_contain(diagnostics, "backend-validate")
    errors += must_contain(policy, "Experimental fallback")
    errors += must_contain(policy, "warning")
    errors += must_contain(policy, "W0001")
    errors += must_contain(policy, "W0002")
    errors += must_contain(policy, "W0004")

    errors += must_contain(wasm_features, "`--emit component` is a hard error")
    errors += must_contain(wasm_features, "T3 `wasm32-wasi-p2` は experimental")
    errors += must_contain(migration, "Component Model | Hard error")
    errors += must_contain(migration, "No `--dir` flag = no filesystem access")

    if errors:
        for error in errors:
            print(error, file=sys.stderr)
        return 1
    print(f"docs consistency OK ({count} fixture entries)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
