#!/usr/bin/env bash
# Verify stdlib manifest matches resolve/typecheck/prelude.ark.
# Exits non-zero if any drift detected.

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MANIFEST="$REPO_ROOT/std/manifest.toml"
CHECKER="$REPO_ROOT/crates/ark-typecheck/src/checker.rs"
PRELUDE="$REPO_ROOT/std/prelude.ark"

python3 - "$MANIFEST" "$CHECKER" "$PRELUDE" <<'PY'
from __future__ import annotations

import re
import sys
import tomllib
from pathlib import Path

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

manifest_path = Path(sys.argv[1])
checker_path = Path(sys.argv[2])
prelude_path = Path(sys.argv[3])

manifest = tomllib.loads(manifest_path.read_text())
functions = manifest.get('functions', [])
manifest_public = sorted({entry['name'] for entry in functions if not entry['name'].startswith('__intrinsic_')})
manifest_all = sorted({entry['name'] for entry in functions} | {entry['intrinsic'] for entry in functions if 'intrinsic' in entry})

checker_text = checker_path.read_text()
checker_names = sorted(set(re.findall(r'fn_sigs\.insert\(\s*\n\s*"([^"]+)"\.into\(\)', checker_text)))
prelude_names = sorted(set(re.findall(r'^pub fn\s+([A-Za-z0-9_]+)\s*\(', prelude_path.read_text(), re.M)))

errors = 0

print(f"{YELLOW}[1/3] Checking prelude.ark fn names vs manifest...{NC}")
missing_prelude = [name for name in prelude_names if name not in manifest_public]
if missing_prelude:
    print(f"{RED}  Functions in prelude.ark but NOT in manifest:{NC}")
    for name in missing_prelude:
        print(f"    {name}")
    errors += 1
else:
    print(f"{GREEN}  ✓ All prelude.ark functions present in manifest{NC}")

extra_manifest = [name for name in manifest_public if name not in prelude_names]
if extra_manifest:
    print(f"{YELLOW}  Functions in manifest but NOT in prelude.ark (may be intentional builtins):{NC}")
    for name in extra_manifest:
        print(f"    {name}")

print(f"{YELLOW}[2/3] Checking checker.rs FnSig names vs manifest...{NC}")
ignored = {'Some', 'Ok', 'Err'}
missing_checker = [name for name in checker_names if name not in manifest_all and name not in ignored]
if missing_checker:
    print(f"{RED}  FnSigs in checker.rs but NOT in manifest:{NC}")
    for name in missing_checker:
        print(f"    {name}")
    errors += 1
else:
    print(f"{GREEN}  ✓ All checker.rs FnSig names present in manifest{NC}")

print(f"{YELLOW}[3/3] Checking intrinsic coverage in manifest...{NC}")
manifest_intrinsics = sorted({entry['intrinsic'] for entry in functions if 'intrinsic' in entry})
if not manifest_intrinsics:
    print(f"{RED}  Manifest does not expose any intrinsic mappings{NC}")
    errors += 1
else:
    print(f"{GREEN}  ✓ Manifest contains intrinsic mappings ({len(manifest_intrinsics)}){NC}")

print(f"{YELLOW}[4/4] Checking stability labels in manifest...{NC}")
# Check that all module-based functions have stability
module_fns = [entry for entry in functions if 'module' in entry]
missing_stability = [entry['name'] for entry in module_fns if 'stability' not in entry]
if missing_stability:
    print(f"{RED}  Module functions missing stability label:{NC}")
    for name in missing_stability:
        print(f"    {name}")
    errors += 1
else:
    stable_count = sum(1 for entry in functions if entry.get('stability') == 'stable')
    exp_count = sum(1 for entry in functions if entry.get('stability') == 'experimental')
    print(f"{GREEN}  ✓ All module functions have stability labels (stable={stable_count}, experimental={exp_count}){NC}")

print()
if errors:
    print(f"{RED}✗ Stdlib manifest check failed ({errors} error(s)){NC}")
    raise SystemExit(1)
print(f"{GREEN}✓ Stdlib manifest is in sync{NC}")
PY
