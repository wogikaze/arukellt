#!/usr/bin/env python3
"""Convert issue files from **Field**: format to YAML frontmatter format."""

import re
import sys
from pathlib import Path

REPO_ROOT = Path.cwd()
print(f"REPO_ROOT: {REPO_ROOT}")
print(f"Script path: {Path(__file__).resolve()}")
ISSUE_DIRS = [
    REPO_ROOT / "issues" / "open",
    REPO_ROOT / "issues" / "done",
    REPO_ROOT / "issues" / "blocked",
    REPO_ROOT / "issues" / "reject",
]

# Fields to extract from header
HEADER_FIELDS = [
    "Status",
    "Created",
    "Updated",
    "ID",
    "Track",
    "Depends on",
    "Orchestration class",
    "Orchestration upstream",
    "Blocks v{N}",
    "Implementation target",
    "ADR candidate",
    "Source",
    "Priority",
    "Status note",
]

def convert_issue(content: str) -> str:
    """Convert issue content from **Field**: format to YAML frontmatter."""
    import frontmatter
    
    # Check if already has frontmatter
    if content.startswith('---'):
        # Parse existing frontmatter and add missing fields
        try:
            post = frontmatter.loads(content)
            frontmatter = dict(post.metadata)
            content_body = post.content
            
            # Also scan body for **Field**: format fields that weren't in frontmatter
            body_lines = content_body.split('\n')
            for line in body_lines:
                match = re.match(r"^\*?\*?\*?([^*:\s][^*]*?)\*?\*?\*?:\s*(.+)$", line.strip())
                if match:
                    field_name = match.group(1).strip()
                    field_value = match.group(2).strip()
                    field_value = re.sub(r'^\*+\s*', '', field_value)
                    field_value = re.sub(r'\s*\*+$', '', field_value)
                    if field_name not in frontmatter:
                        frontmatter[field_name] = field_value
            
            # Add missing required fields with defaults
            modified = False
            if 'Track' not in frontmatter:
                frontmatter['Track'] = 'main'
                modified = True
            if 'Orchestration class' not in frontmatter:
                frontmatter['Orchestration class'] = 'implementation-ready'
                modified = True
            if 'Depends on' not in frontmatter:
                frontmatter['Depends on'] = 'none'
                modified = True
            
            if not modified:
                return content
            
            # Rebuild with updated frontmatter
            yaml_lines = ['---']
            for field_name, value in frontmatter.items():
                if ':' in str(value) or '[' in str(value) or ']' in str(value) or '(' in str(value):
                    yaml_lines.append(f'{field_name}: "{value}"')
                else:
                    yaml_lines.append(f'{field_name}: {value}')
            yaml_lines.append('---')
            yaml_lines.append('')
            
            return '\n'.join(yaml_lines) + content_body
        except Exception as e:
            print(f"Warning: Failed to parse existing frontmatter: {e}")
            return content
    else:
        # Convert from **Field**: format to frontmatter
        lines = content.split('\n')
        
        # Extract title (first line starting with #)
        title_line = None
        title_idx = 0
        for i, line in enumerate(lines):
            if line.startswith('#'):
                title_line = line
                title_idx = i
                break
        
        if not title_line:
            print(f"Warning: No title found in file")
            return content
        
        # Extract header fields
        frontmatter = {}
        header_end_idx = title_idx + 1
        
        for i in range(title_idx + 1, len(lines)):
            line = lines[i].strip()
            if not line:
                continue
            
            # Check if this is a header field
            match = re.match(r"^\*?\*?\*?([^*:\s][^*]*?)\*?\*?\*?:\s*(.+)$", line)
            if match:
                field_name = match.group(1).strip()
                field_value = match.group(2).strip()
                
                # Clean up field value (remove markdown bold markers)
                field_value = re.sub(r'^\*+\s*', '', field_value)
                field_value = re.sub(r'\s*\*+$', '', field_value)
                
                # Store all fields
                frontmatter[field_name] = field_value
                header_end_idx = i + 1
            elif line.startswith('##'):
                # Stop at first section header
                break
            elif line.startswith('---'):
                # Already has frontmatter, skip
                return content
        
        if not frontmatter:
            print(f"Warning: No header fields found")
            return content
        
        # Add missing required fields with defaults
        if 'Track' not in frontmatter:
            frontmatter['Track'] = 'main'
        if 'Orchestration class' not in frontmatter:
            frontmatter['Orchestration class'] = 'implementation-ready'
        if 'Depends on' not in frontmatter:
            frontmatter['Depends on'] = 'none'
        
        # Build YAML frontmatter
        yaml_lines = ['---']
        for field_name, value in frontmatter.items():
            if ':' in str(value) or '[' in str(value) or ']' in str(value) or '(' in str(value):
                yaml_lines.append(f'{field_name}: "{value}"')
            else:
                yaml_lines.append(f'{field_name}: {value}')
        yaml_lines.append('---')
        yaml_lines.append('')
        
        # Build new content
        new_lines = yaml_lines + [title_line] + lines[header_end_idx:]
        
        return '\n'.join(new_lines)

def main():
    """Convert all issue files."""
    converted = 0
    skipped = 0
    errors = 0
    
    print(f"Checking directories:")
    for issue_dir in ISSUE_DIRS:
        print(f"  {issue_dir}: exists={issue_dir.exists()}")
    
    for issue_dir in ISSUE_DIRS:
        if not issue_dir.exists():
            continue
        
        print(f"\nProcessing {issue_dir}...")
        md_files = list(issue_dir.glob("*.md"))
        print(f"  Found {len(md_files)} .md files")
        
        for issue_file in md_files:
            # Skip non-issue files
            if issue_file.name in ["index.md", "dependency-graph.md", "priority-table.md"]:
                skipped += 1
                continue
            # Skip placeholder files
            if "placeholder" in issue_file.name.lower():
                skipped += 1
                continue
            
            content = issue_file.read_text()
            
            # Don't skip if already has frontmatter - we want to add missing fields
            # if content.startswith('---'):
            #     skipped += 1
            #     continue
            
            # Convert
            try:
                new_content = convert_issue(content)
                if new_content != content:
                    issue_file.write_text(new_content)
                    converted += 1
                    print(f"Converted: {issue_file.relative_to(REPO_ROOT)}")
                else:
                    skipped += 1
            except Exception as e:
                errors += 1
                print(f"Error converting {issue_file.relative_to(REPO_ROOT)}: {e}", file=sys.stderr)
    
    print(f"\nSummary:")
    print(f"  Converted: {converted}")
    print(f"  Skipped: {skipped}")
    print(f"  Errors: {errors}")
    
    if errors > 0:
        sys.exit(1)

if __name__ == "__main__":
    main()
