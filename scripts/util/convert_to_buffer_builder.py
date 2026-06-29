#!/usr/bin/env python3
"""Convert text::builder_append call sites from String-based to Vec<i32>-based builder API.

Transforms:
1. let mut VAR = String_new()  →  let mut VAR = text::builder_new()
2. let mut VAR = ""             →  let mut VAR = text::builder_new()
3. let mut VAR = String_from(X) →  let mut VAR = text::builder_new() + insert append
4. Consumption points: wrap VAR with text::builder_build(VAR) where String is expected

Does NOT touch:
- VAR = text::builder_append(VAR, X)  (builder_append now takes Vec<i32>)
- text::builder_build(VAR)            (already wrapped)
"""

import re
import os
import sys

def find_builder_vars(lines):
    """Find variables used with text::builder_append(VAR, ...)"""
    vars_set = set()
    for line in lines:
        m = re.search(r'(\w+) = text::builder_append\(\1,', line)
        if m:
            vars_set.add(m.group(1))
        # Also check: text::builder_append(VAR, ...) without assignment
        m2 = re.search(r'text::builder_append\((\w+),', line)
        if m2:
            vars_set.add(m2.group(1))
    return vars_set

def has_text_import(lines):
    for line in lines:
        if 'use std::text' in line:
            return True
    return False

def add_text_import(lines):
    last_use_idx = -1
    for i, line in enumerate(lines):
        if line.strip().startswith('use '):
            last_use_idx = i
    if last_use_idx >= 0:
        lines.insert(last_use_idx + 1, 'use std::text')
        next_idx = last_use_idx + 2
        if next_idx < len(lines) and lines[next_idx].strip() != '':
            lines.insert(next_idx, '')
    else:
        insert_idx = 0
        for i, line in enumerate(lines):
            if line.strip().startswith('//') or line.strip() == '':
                insert_idx = i + 1
            else:
                break
        lines.insert(insert_idx, 'use std::text')
        lines.insert(insert_idx + 1, '')
    return lines

def convert_file(filepath):
    with open(filepath, 'r') as f:
        content = f.read()
    lines = content.split('\n')
    
    builder_vars = find_builder_vars(lines)
    if not builder_vars:
        return False
    
    if not has_text_import(lines):
        lines = add_text_import(lines)
    
    changed = False
    insert_lines = {}  # line_index -> list of lines to insert after
    
    # Step 1: Fix initializations
    for i, line in enumerate(lines):
        stripped = line.strip()
        for var in builder_vars:
            # let mut VAR = String_new()
            if f'let mut {var} = String_new()' in line:
                lines[i] = line.replace(f'let mut {var} = String_new()', f'let mut {var} = text::builder_new()')
                changed = True
            # let mut VAR = ""
            elif f'let mut {var} = ""' in line:
                lines[i] = line.replace(f'let mut {var} = ""', f'let mut {var} = text::builder_new()')
                changed = True
            # let mut VAR = String_from(X)
            elif f'let mut {var} = String_from(' in line:
                # Extract the String_from argument
                m = re.search(f'let mut {var} = String_from\\((.+)\\)', line)
                if m:
                    arg = m.group(1)
                    indent = line[:len(line) - len(line.lstrip())]
                    lines[i] = f'{indent}let mut {var} = text::builder_new()'
                    insert_lines.setdefault(i, []).append(f'{indent}{var} = text::builder_append({var}, String_from({arg}))')
                    changed = True
    
    # Apply insertions (in reverse order to preserve indices)
    for idx in sorted(insert_lines.keys(), reverse=True):
        for insert_line in reversed(insert_lines[idx]):
            lines.insert(idx + 1, insert_line)
    
    # Step 2: Wrap consumption points with builder_build
    for i, line in enumerate(lines):
        stripped = line.strip()
        for var in builder_vars:
            # Skip builder_append calls (VAR is Vec<i32> here)
            if f'text::builder_append({var},' in line:
                continue
            # Skip builder_build calls (already wrapped)
            if f'text::builder_build({var})' in line:
                continue
            # Skip builder_new
            if f'text::builder_new()' in line:
                continue
            # Skip initialization
            if f'let mut {var} = ' in line:
                continue
            
            # return VAR  →  return text::builder_build(VAR)
            if stripped == f'return {var}':
                indent = line[:len(line) - len(line.lstrip())]
                lines[i] = f'{indent}return text::builder_build({var})'
                changed = True
            # bare VAR as return value (last expression in function)
            elif stripped == var:
                indent = line[:len(line) - len(line.lstrip())]
                lines[i] = f'{indent}text::builder_build({var})'
                changed = True
            
            # concat(VAR, X)  →  concat(text::builder_build(VAR), X)
            if f'concat({var},' in line or f'concat({var}, ' in line:
                lines[i] = line.replace(f'concat({var},', f'concat(text::builder_build({var}),')
                lines[i] = lines[i].replace(f'concat({var}, ', f'concat(text::builder_build({var}), ')
                changed = True
            
            # __intrinsic_concat(VAR, X)  →  __intrinsic_concat(text::builder_build(VAR), X)
            if f'__intrinsic_concat({var},' in line:
                lines[i] = line.replace(f'__intrinsic_concat({var},', f'__intrinsic_concat(text::builder_build({var}),')
                changed = True
            
            # push(VEC, VAR)  →  push(VEC, text::builder_build(VAR))
            # Match push(something, VAR) where VAR is at end of push call
            if re.search(f'push\\([^,]+, {var}\\)', line):
                lines[i] = re.sub(f'push\\(([^,]+), {var}\\)', f'push(\\1, text::builder_build({var}))', line)
                changed = True
            
            # eq(VAR, X) or eq(clone(VAR), X)  →  eq(text::builder_build(VAR), X)
            if f'eq(clone({var}),' in line:
                lines[i] = line.replace(f'eq(clone({var}),', f'eq(text::builder_build({var}),')
                changed = True
            elif f'eq({var},' in line and 'eq(clone' not in line:
                lines[i] = line.replace(f'eq({var},', f'eq(text::builder_build({var}),')
                changed = True
            
            # starts_with(VAR, X)  →  starts_with(text::builder_build(VAR), X)
            if f'starts_with({var},' in line:
                lines[i] = line.replace(f'starts_with({var},', f'starts_with(text::builder_build({var}),')
                changed = True
            
            # index_of(VAR, X)  →  index_of(text::builder_build(VAR), X)
            if f'index_of({var},' in line:
                lines[i] = line.replace(f'index_of({var},', f'index_of(text::builder_build({var}),')
                changed = True
            
            # contains(VAR, X)  →  contains(text::builder_build(VAR), X)
            if f'contains({var},' in line:
                lines[i] = line.replace(f'contains({var},', f'contains(text::builder_build({var}),')
                changed = True
            
            # substring(VAR, ...)  →  substring(text::builder_build(VAR), ...)
            if f'substring({var},' in line:
                lines[i] = line.replace(f'substring({var},', f'substring(text::builder_build({var}),')
                changed = True
            
            # char_at(VAR, ...)  →  char_at(text::builder_build(VAR), ...)
            if f'char_at({var},' in line:
                lines[i] = line.replace(f'char_at({var},', f'char_at(text::builder_build({var}),')
                changed = True
            
            # len(VAR) is fine — Vec has len too, no change needed
            
            # String_from(VAR) — shouldn't happen but just in case
            if f'String_from({var})' in line:
                lines[i] = line.replace(f'String_from({var})', f'text::builder_build({var})')
                changed = True
    
    if changed:
        with open(filepath, 'w') as f:
            f.write('\n'.join(lines))
        print(f'  converted: {filepath}')
    
    return changed

def main():
    os.chdir('/home/wogikaze/arukellt')
    
    # Find all .ark files with text::builder_append
    files = []
    for root, dirs, fnames in os.walk('src/compiler/'):
        for fname in fnames:
            if not fname.endswith('.ark'):
                continue
            filepath = os.path.join(root, fname)
            with open(filepath) as f:
                content = f.read()
            if 'text::builder_append' in content:
                files.append(filepath)
    
    print(f'Found {len(files)} files with text::builder_append')
    total = 0
    for f in sorted(files):
        if convert_file(f):
            total += 1
    print(f'\nTotal files converted: {total}')

if __name__ == '__main__':
    main()
