#!/usr/bin/env python3
"""Fix YAML frontmatter errors by removing body content from frontmatter."""

import re
import sys
from pathlib import Path

try:
    import frontmatter
except ImportError:
    print("ERROR: frontmatter library not installed. Install with: pip install python-frontmatter")
    sys.exit(1)

REPO_ROOT = Path.cwd()
ISSUE_DIRS = [
    REPO_ROOT / "issues" / "done",
    REPO_ROOT / "issues" / "open",
    REPO_ROOT / "issues" / "blocked",
    REPO_ROOT / "issues" / "reject",
]

def fix_frontmatter(content: str) -> str:
    """Fix YAML frontmatter by removing invalid content."""
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
        return content
    
    # Extract frontmatter lines
    frontmatter_lines = lines[1:frontmatter_end]
    
    # Filter out invalid lines (lines that don't look like YAML key-value pairs)
    valid_frontmatter = []
    for line in frontmatter_lines:
        stripped = line.strip()
        if not stripped:
            valid_frontmatter.append(line)
        elif ':' in stripped and not stripped.startswith('#'):
            # Check if it looks like a valid YAML key-value pair
            key_part = stripped.split(':', 1)[0].strip()
            if key_part and not key_part[0].islower() or key_part in ['ID', 'Status', 'Created', 'Updated', 'Track', 'Depends on']:
                valid_frontmatter.append(line)
        else:
            # Invalid line, skip it (it belongs in body)
            pass
    
    # Rebuild content
    new_lines = ['---'] + valid_frontmatter + ['---'] + lines[frontmatter_end + 1:]
    
    return '\n'.join(new_lines)

def main():
    """Fix all issue files with YAML errors."""
    fixed = 0
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
            
            try:
                # Try to parse with frontmatter
                post = frontmatter.load(issue_file)
            except:
                # Parsing failed, try to fix
                content = issue_file.read_text()
                try:
                    new_content = fix_frontmatter(content)
                    # Try to parse the fixed content
                    frontmatter.loads(new_content)
                    # If successful, write back
                    issue_file.write_text(new_content)
                    fixed += 1
                    print(f"Fixed: {issue_file.relative_to(REPO_ROOT)}")
                except Exception as e:
                    errors += 1
                    print(f"Failed to fix {issue_file.relative_to(REPO_ROOT)}: {e}", file=sys.stderr)
            else:
                skipped += 1
    
    print(f"\nSummary:")
    print(f"  Fixed: {fixed}")
    print(f"  Skipped: {skipped}")
    print(f"  Errors: {errors}")
    
    if errors > 0:
        sys.exit(1)

if __name__ == "__main__":
    main()
