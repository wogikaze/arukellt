#!/usr/bin/env python3
from pathlib import Path
import json
import tomllib

root = Path(__file__).resolve().parents[2]
data = tomllib.loads((root / "data/core-ops.toml").read_text(encoding="utf-8"))
rows = []
for op in data.get("operations", []):
    if op.get("classification", {}).get("layer") != "runtime":
        continue
    lowering = op.get("lowering", {})
    rows.append({
        "id": op.get("id"),
        "lowering_kind": lowering.get("kind"),
        "runtime": lowering.get("runtime", {}),
    })
print("RUNTIME_PAYLOADS=" + json.dumps(rows, sort_keys=True))
