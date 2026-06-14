#!/usr/bin/env python3
from __future__ import annotations
from pathlib import Path
R=Path(__file__).resolve().parents[2]; O=R/"issues"/"open"
def issue_is_open(iid):
    p=f"{int(iid):03d}-"; return any(x.name.startswith(p) for x in O.glob("*.md"))
def skip_if_any_open(ids, name):
    o=[i for i in ids if issue_is_open(i)]
    if o: print(f"{name}: SKIP ({', '.join('#'+i for i in o)} open)"); return 0
    return None
