#!/usr/bin/env python3
"""Validate issue header fields are present and have valid values."""

import os
import re
import sys
from pathlib import Path

try:
    import frontmatter
except ImportError:
    print("ERROR: frontmatter library not installed. Install with: pip install python-frontmatter")
    sys.exit(1)

REPO_ROOT = Path(__file__).resolve().parents[2]

# Only validate open issues by default (done/blocked/reject may have legacy issues)
# Set VALIDATE_ALL=true to validate all directories
VALIDATE_ALL = os.environ.get("VALIDATE_ALL", "").lower() == "true"

ISSUE_DIRS = [REPO_ROOT / "issues" / "open"]
if VALIDATE_ALL:
    ISSUE_DIRS.extend([
        REPO_ROOT / "issues" / "done",
        REPO_ROOT / "issues" / "blocked",
        REPO_ROOT / "issues" / "reject",
    ])

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
    # Additional values found in existing issues
    "selfhost-backend",
    "formatter",
    "docs",
    "selfhost-cli",
    "lsp-semantic",
    "ci",
    "cli",
    "corehir",
    "lowering",
    "docs-audit",
    "planning",
    "release",
    "component",
}

# Valid values for Orchestration class field
VALID_ORCHESTRATION_CLASSES = {
    "implementation-ready",
    "design-ready",
    "verification-ready",
    "blocked-by-upstream",
    "unsupported-in-this-run",
}

# Pattern for Depends on field: "none", "—" (em dash), or comma-separated IDs (with optional suffixes like 028b)
DEPENDS_ON_PATTERN = re.compile(r"^(none|—|(\d{3}[a-z]?(,\s*\d{3}[a-z]?)*)(\s*\(.*\))?)$")

errors = []
warnings = []

for issue_dir in ISSUE_DIRS:
    if not issue_dir.exists():
        continue
    
    for issue_file in issue_dir.glob("*.md"):
        # Skip non-issue files
        if issue_file.name in ["index.md", "dependency-graph.md", "priority-table.md"]:
            continue
        # Skip placeholder files
        if "placeholder" in issue_file.name.lower():
            continue
        
        try:
            post = frontmatter.load(issue_file)
        except Exception as e:
            errors.append(f"{issue_file.relative_to(REPO_ROOT)}: failed to parse frontmatter: {e}")
            continue
        
        # Skip operational guide umbrella issues (they have different format)
        content = post.content
        if "Operational Guide" in content or "Implementation Guide" in content:
            continue
        # Skip release checklist issues (they have different format with > **Track:**)
        if content.startswith("# Release:") or content.startswith("# Release:"):
            continue
        # Skip done issues in open directory
        status_val = post.get('Status', '').lower()
        if status_val == "done":
            continue
        
        # Extract fields from frontmatter
        track_value = post.get('Track')
        orch_value = post.get('Orchestration class')
        depends_value = post.get('Depends on')
        
        # Check Track field
        if not track_value:
            errors.append(f"{issue_file.relative_to(REPO_ROOT)}: missing Track field")
        else:
            # Handle combined tracks like "selfhost-frontend / language-design"
            track_parts = [t.strip() for t in str(track_value).split("/")]
            for part in track_parts:
                if part not in VALID_TRACKS:
                    errors.append(f"{issue_file.relative_to(REPO_ROOT)}: invalid Track value: {part}")
        
        # Check Orchestration class field
        if not orch_value:
            errors.append(f"{issue_file.relative_to(REPO_ROOT)}: missing Orchestration class field")
        else:
            if orch_value not in VALID_ORCHESTRATION_CLASSES:
                errors.append(f"{issue_file.relative_to(REPO_ROOT)}: invalid Orchestration class value: {orch_value}")
        
        # Check Depends on field
        if not depends_value:
            errors.append(f"{issue_file.relative_to(REPO_ROOT)}: missing Depends on field")
        else:
            if not DEPENDS_ON_PATTERN.match(str(depends_value)):
                errors.append(f"{issue_file.relative_to(REPO_ROOT)}: invalid Depends on value: {depends_value}")

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
