#!/usr/bin/env python3
"""Remove **Field**: format fields from issue body content."""

import re
import sys
from pathlib import Path

REPO_ROOT = Path.cwd()
ISSUE_DIRS = [
    REPO_ROOT / "issues" / "done",
    REPO_ROOT / "issues" / "open",
    REPO_ROOT / "issues" / "blocked",
    REPO_ROOT / "issues" / "reject",
]

def cleanup_body(content: str) -> str:
    """Remove **Field**: format fields from body content."""
    lines = content.split('\n')
    
    # Find the end of frontmatter (second ---)
    frontmatter_end = -1
    dash_count = 0
    for i, line in enumerate(lines):
        if line.strip() == '---':
            dash_count += 1
            if dash_count == 2:
                frontmatter_end = i
                break
    
    if frontmatter_end == -1:
        # No frontmatter, remove **Field**: lines from entire content
        body_lines = lines
    else:
        # Keep frontmatter, process body
        body_lines = lines[frontmatter_end + 1:]
    
    # Remove **Field**: format lines from body
    cleaned_body = []
    field_pattern = re.compile(r"^\s*\*+\s*([^*:\s][^*]*?)\s*\*+\s*:\s*.+$")
    
    for line in body_lines:
        if not field_pattern.match(line):
            cleaned_body.append(line)
    
    if frontmatter_end == -1:
        return '\n'.join(cleaned_body)
    else:
        return '\n'.join(lines[:frontmatter_end + 1] + cleaned_body)

def main():
    """Clean up body fields in all issue files."""
    cleaned = 0
    skipped = 0
    errors = 0
    
    for issue_dir in ISSUE_DIRS:
        if not issue_dir.exists():
            continue
        
        md_files = list(issue_dir.glob("*.md"))
        
        for issue_file in md_files:
            if issue_file.name in ["index.md", "dependency-graph.md", "priority-table.md"]:
                continue
            if "placeholder" in issue_file.name.lower():
                continue
            
            content = issue_file.read_text()
            new_content = cleanup_body(content)
            
            if new_content != content:
                issue_file.write_text(new_content)
                cleaned += 1
                print(f"Cleaned: {issue_file.relative_to(REPO_ROOT)}")
            else:
                skipped += 1
    
    print(f"\nSummary:")
    print(f"  Cleaned: {cleaned}")
    print(f"  Skipped: {skipped}")
    print(f"  Errors: {errors}")

if __name__ == "__main__":
    main()
