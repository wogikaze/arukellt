#!/usr/bin/env python3
"""Fix YAML frontmatter errors in done files by moving invalid content to body."""

import re
import sys
from pathlib import Path

REPO_ROOT = Path.cwd()
DONE_DIR = REPO_ROOT / "issues" / "done"

def fix_yaml_frontmatter(content: str) -> str:
    """Fix YAML frontmatter by moving invalid content to body."""
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
        return content  # No frontmatter, return as-is
    
    # Extract frontmatter lines
    frontmatter_lines = lines[1:frontmatter_end]
    
    # Filter out invalid lines and move them to body
    valid_frontmatter = []
    invalid_lines = []
    
    for line in frontmatter_lines:
        stripped = line.strip()
        if not stripped:
            valid_frontmatter.append(line)
        elif ':' in stripped:
            # Check if it looks like a valid YAML key-value pair
            key_part = stripped.split(':', 1)[0].strip()
            # Allow keys that are uppercase or known field names
            if key_part and (key_part[0].isupper() or key_part in ['ID', 'Status', 'Created', 'Updated', 'Track', 'Depends on', 'Orchestration class', 'Orchestration upstream', 'Blocks v{N}', 'Implementation target', 'ADR candidate', 'Source', 'Priority', 'Status note']):
                valid_frontmatter.append(line)
            else:
                invalid_lines.append(line)
        else:
            invalid_lines.append(line)
    
    # Rebuild content
    new_lines = ['---'] + valid_frontmatter + ['---']
    if invalid_lines:
        new_lines.append('')  # Add blank line before invalid content
        new_lines.extend(invalid_lines)
    new_lines.extend(lines[frontmatter_end + 1:])
    
    return '\n'.join(new_lines)

def main():
    """Fix all done files with YAML errors."""
    fixed = 0
    skipped = 0
    errors = 0
    
    try:
        import frontmatter
    except ImportError:
        print("ERROR: frontmatter library not installed")
        sys.exit(1)
    
    md_files = sorted(DONE_DIR.glob('*.md'))
    
    for issue_file in md_files:
        if issue_file.name in ["index.md", "dependency-graph.md", "priority-table.md"]:
            continue
        if "placeholder" in issue_file.name.lower():
            continue
        
        try:
            # Try to parse
            frontmatter.load(issue_file)
            skipped += 1
        except:
            # Parsing failed, try to fix
            content = issue_file.read_text()
            try:
                new_content = fix_yaml_frontmatter(content)
                # Try to parse the fixed content
                frontmatter.loads(new_content)
                # If successful, write back
                issue_file.write_text(new_content)
                fixed += 1
                print(f"Fixed: {issue_file.name}")
            except Exception as e:
                errors += 1
                print(f"Failed to fix {issue_file.name}: {e}", file=sys.stderr)
    
    print(f"\nSummary:")
    print(f"  Fixed: {fixed}")
    print(f"  Skipped: {skipped}")
    print(f"  Errors: {errors}")

if __name__ == "__main__":
    main()
