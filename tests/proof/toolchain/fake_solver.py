#!/usr/bin/env python3
from __future__ import annotations

import sys

source = sys.stdin.read()
if "prove" in source:
    print("unsat")
elif "refute" in source:
    print("sat")
elif "error" in source:
    print("(error simulated)")
    raise SystemExit(2)
else:
    print("unknown")
