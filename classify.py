import os
import re

open_dir = '/home/wogikaze/arukellt/issues/open'
done_dir = '/home/wogikaze/arukellt/issues/done'

done_ids = set()
for f in os.listdir(done_dir):
    if f.endswith('.md'):
        m = re.match(r'^(\d+)', f)
        if m:
            done_ids.add(m.group(1))

dag_path = os.path.join(open_dir, 'dependency-graph.md')
deps_map = {}
with open(dag_path, 'r') as f:
    for line in f:
        line = line.strip()
        if line.startswith('- **') and 'depends on:' in line:
            m = re.match(r'- \*\*(\d+)\*\*.*depends on: ([^;]+);', line)
            if m:
                id_val = m.group(1)
                deps_str = m.group(2)
                deps = []
                if deps_str != 'none':
                    for d in deps_str.split(','):
                        d = d.strip()
                        md = re.search(r'\d+', d)
                        if md:
                            deps.append(md.group(0).zfill(3))
                deps_map[id_val] = deps

index_path = os.path.join(open_dir, 'index.md')
issues = {}
with open(index_path, 'r') as f:
    for line in f:
        if line.startswith('| '):
            parts = [p.strip() for p in line.split('|')]
            if len(parts) >= 10 and re.match(r'^\d+$', parts[1]):
                id_val = parts[1]
                idx_state = parts[8]
                title = parts[2]
                issues[id_val] = {
                    'idx_state': idx_state,
                    'deps': deps_map.get(id_val, []),
                    'title': title
                }

classified = {
    'implementation-ready': [],
    'design-ready': [],
    'verification-ready': [],
    'blocked-by-upstream': [],
    'unsupported-in-this-run': []
}

for id_val, data in sorted(issues.items()):
    is_blocked = False
    for d in data['deps']:
        if d not in done_ids:
            is_blocked = True
            break
            
    if is_blocked:
        classified['blocked-by-upstream'].append(id_val)
    else:
        state = data['idx_state']
        if state == 'blocked-by-upstream':
            print(f"UNBLOCKED! {id_val}")
            classified['implementation-ready'].append(id_val)
        elif state in classified:
            classified[state].append(id_val)
        else:
            classified['implementation-ready'].append(id_val)

for k, v in classified.items():
    print(f"=== {k} ===")
    for id_val in v:
        print(f"{id_val} {issues[id_val]['title']}")
