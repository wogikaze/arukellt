#!/usr/bin/env python3
"""Fix remaining YAML errors by moving everything after the first non-YAML line to body."""

import re
import sys
from pathlib import Path

REPO_ROOT = Path.cwd()
DONE_DIR = REPO_ROOT / "issues" / "done"

ERROR_FILES = [
    "006-vec-on-wasmgc.md",
    "021-gc-native-structs.md",
    "022-gc-native-enums.md",
    "028b-wit-import-pipeline-wiring.md",
    "039-module-system-infrastructure.md",
    "094-t3-br-on-cast-chain-opt.md",
    "122-compile-mir-opt-level-separation.md",
    "208-selfhost-module-private-items-in-scope.md",
    "260-current-state-target-table-from-ci.md",
    "271-vscode-test-runner-wiring.md",
    "331-perf-baseline-selfhost.md",
    "447-std-host-sockets-implementation.md",
    "451-lsp-hover-semantic-only.md",
    "453-vscode-e2e-editor-behavior-tests.md",
    "456-arukellt-doc-command.md",
    "459-selfhost-fixpoint-dual-period-end.md",
    "462-extension-settings-rationalization.md",
    "472-playground-type-checker-product-claim.md",
    "487-package-registry-resolution.md",
    "490-pub-use-pub-import-re-export.md",
    "493-selfhost-mir-control-flow-coverage.md",
    "494-selfhost-mir-ssa-formation.md",
    "501-t2-wasm-emit-implementation.md",
    "504-selfhost-trait-syntax.md",
    "515-stdlib-result-option-sentinel-cleanup.md",
    "517-stdlib-api-consistency-and-canonical-naming-v2.md",
    "521-stdlib-json-top-level-parse-contract.md",
    "530-selfhost-parity-ci-gates.md",
    "559-phase5-selfhost-first-execution-path.md",
    "570-phase6-hover--definition-handlers.md",
    "572-phase7-delete-ark-lsp.md",
    "583-phase5-prereq-retire-arukellt-rust-legacy.md",
]

def fix_yaml_frontmatter(content: str) -> str:
    """Fix YAML by moving everything after first invalid line to body."""
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
    
    # Find first invalid line
    valid_frontmatter = []
    invalid_lines = []
    found_invalid = False
    
    for line in frontmatter_lines:
        stripped = line.strip()
        if not stripped:
            if not found_invalid:
                valid_frontmatter.append(line)
            else:
                invalid_lines.append(line)
        elif ':' in stripped:
            key_part = stripped.split(':', 1)[0].strip()
            # Allow keys that are uppercase or known field names
            if key_part and (key_part[0].isupper() or key_part in ['ID', 'Status', 'Created', 'Updated', 'Track', 'Depends on', 'Orchestration class', 'Orchestration upstream', 'Blocks v{N}', 'Implementation target', 'ADR candidate', 'Source', 'Priority', 'Status note']):
                if found_invalid:
                    invalid_lines.append(line)
                else:
                    valid_frontmatter.append(line)
            else:
                found_invalid = True
                invalid_lines.append(line)
        else:
            found_invalid = True
            invalid_lines.append(line)
    
    # Rebuild content
    new_lines = ['---'] + valid_frontmatter + ['---']
    if invalid_lines:
        new_lines.append('')  # Add blank line before invalid content
        new_lines.extend(invalid_lines)
    new_lines.extend(lines[frontmatter_end + 1:])
    
    return '\n'.join(new_lines)

def main():
    """Fix remaining done files with YAML errors."""
    fixed = 0
    errors = 0
    
    try:
        import frontmatter
    except ImportError:
        print("ERROR: frontmatter library not installed")
        sys.exit(1)
    
    for filename in ERROR_FILES:
        issue_file = DONE_DIR / filename
        if not issue_file.exists():
            print(f"Skipping (not found): {filename}")
            continue
        
        try:
            # Try to parse
            frontmatter.load(issue_file)
            print(f"Already valid: {filename}")
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
                print(f"Fixed: {filename}")
            except Exception as e:
                errors += 1
                print(f"Failed to fix {filename}: {e}", file=sys.stderr)
    
    print(f"\nSummary:")
    print(f"  Fixed: {fixed}")
    print(f"  Errors: {errors}")

if __name__ == "__main__":
    main()
