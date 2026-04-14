#!/usr/bin/env python3
"""Classify open issues by dependency readiness."""
import os, re, sys

done_dir = "issues/done"
open_dir = "issues/open"

done_ids = set()
for f in os.listdir(done_dir):
    m = re.match(r'^0*(\d+)-', f)
    if m: done_ids.add(m.group(1))

issues = {}
for f in sorted(os.listdir(open_dir)):
    if not f.endswith('.md'): continue
    m = re.match(r'^0*(\d+)-', f)
    if not m: continue
    iid = m.group(1)
    text = open(os.path.join(open_dir, f)).read()
    dep_m = re.search(r'\*\*Depends on\*\*:\s*(.+)', text)
    deps = []
    if dep_m:
        raw = dep_m.group(1).strip()
        if raw.lower() not in ('none', '-', ''):
            for part in raw.split(','):
                part = part.strip()
                nm = re.match(r'^#?0*(\d+)', part)
                if nm: deps.append(nm.group(1))
    track_m = re.search(r'\*\*Track\*\*:\s*(.+)', text)
    track = track_m.group(1).strip() if track_m else 'unknown'
    issues[iid] = {'file': f, 'track': track, 'deps': deps}

for iid, info in issues.items():
    info['unmet'] = [d for d in info['deps'] if d not in done_ids]

print("=== IMPLEMENTATION READY ===")
for iid in sorted(issues, key=int):
    info = issues[iid]
    if not info['unmet']:
        print(f"  {iid:>4} | {info['track']:<22} | {info['file']}")

print()
print("=== BLOCKED ===")
for iid in sorted(issues, key=int):
    info = issues[iid]
    if info['unmet']:
        print(f"  {iid:>4} | {info['track']:<22} | unmet={info['unmet']}")
