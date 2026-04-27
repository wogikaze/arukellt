#!/usr/bin/env python3
"""Validate issue header fields are present and have valid values."""

import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
ISSUE_DIRS = [
    REPO_ROOT / "issues" / "open",
    REPO_ROOT / "issues" / "done",
    REPO_ROOT / "issues" / "blocked",
    REPO_ROOT / "issues" / "reject",
]

# Valid values for Track field (collected from existing issues)
VALID_TRACKS = {
    "component-model",
    "wasi-feature",
    "wasm-feature",
    "selfhost-retirement",
    "selfhost-frontend",
    "selfhost",
    "typechecker",
    "compiler",
    "runtime-perf",
    "parallel",
    "main",
    "stdlib",
    "stdlib-api",
    "stdlib-docs",
    "playground",
    "compile-speed",
    "mir-opt",
    "backend-opt",
    "wasm-quality",
    "vscode-ide",
    "language-docs",
    "language-design",
    "repo-hygiene",
    "runtime",
    "pipeline-refactor",
    "gc-native",
    "benchmark",
    "extension",
    "code-structure",
    "cross-cutting",
    "capability",
    "diagnostics",
    "lsp-navigation",
    "maintenance",
    "selfhost-verification",
    "linter",
}

# Valid values for Orchestration class field
VALID_ORCHESTRATION_CLASSES = {
    "implementation-ready",
    "design-ready",
    "verification-ready",
    "blocked-by-upstream",
    "unsupported-in-this-run",
}

# Pattern for Depends on field: "none" or comma-separated IDs
DEPENDS_ON_PATTERN = re.compile(r"^(none|(\d{3}(,\s*\d{3})*))$")

errors = []
warnings = []

for issue_dir in ISSUE_DIRS:
    if not issue_dir.exists():
        continue
    
    for issue_file in issue_dir.glob("*.md"):
        content = issue_file.read_text()
        
        # Extract header fields
        track_match = re.search(r"^\*\*Track\*\*:\s*(.+)$", content, re.MULTILINE)
        orch_match = re.search(r"^\*\*Orchestration class\*\*:\s*(.+)$", content, re.MULTILINE)
        depends_match = re.search(r"^\*\*Depends on\*\*:\s*(.+)$", content, re.MULTILINE)
        
        # Check Track field
        if not track_match:
            errors.append(f"{issue_file.relative_to(REPO_ROOT)}: missing **Track** field")
        else:
            track_value = track_match.group(1).strip()
            # Handle combined tracks like "selfhost-frontend / language-design"
            track_parts = [t.strip() for t in track_value.split("/")]
            for part in track_parts:
                if part not in VALID_TRACKS:
                    errors.append(f"{issue_file.relative_to(REPO_ROOT)}: invalid **Track** value: {part}")
        
        # Check Orchestration class field
        if not orch_match:
            errors.append(f"{issue_file.relative_to(REPO_ROOT)}: missing **Orchestration class** field")
        else:
            orch_value = orch_match.group(1).strip()
            if orch_value not in VALID_ORCHESTRATION_CLASSES:
                errors.append(f"{issue_file.relative_to(REPO_ROOT)}: invalid **Orchestration class** value: {orch_value}")
        
        # Check Depends on field
        if not depends_match:
            errors.append(f"{issue_file.relative_to(REPO_ROOT)}: missing **Depends on** field")
        else:
            depends_value = depends_match.group(1).strip()
            if not DEPENDS_ON_PATTERN.match(depends_value):
                errors.append(f"{issue_file.relative_to(REPO_ROOT)}: invalid **Depends on** value: {depends_value}")

if errors:
    print("ERROR: Issue header validation failed")
    for error in errors:
        print(f"  {error}")
    sys.exit(1)

if warnings:
    print("WARNINGS:")
    for warning in warnings:
        print(f"  {warning}")

print("OK: All issue headers are valid")
sys.exit(0)
