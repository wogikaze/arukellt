#!/usr/bin/env python3
"""Check final Phase 5 boundary including source schema and execution entrypoints."""
from __future__ import annotations
import hashlib, json, subprocess, sys
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / "scripts/check/check-proof-phase5-boundary-receipt.py"
REQUIRED_PATHS = {
    "schemas/typed-corehir-v2.schema.json",
    "scripts/gen/convert-typed-corehir-v2.py",
    "scripts/gen/write-smt-vcs-v5.py",
    "scripts/gen/prepare-proof-phase5-toolchain.py",
    "scripts/check/check-proof-phase5-boundary-receipt.py",
    ".github/workflows/proof-phase5.yml",
}

def sha(path: Path) -> str:
    d = hashlib.sha256(); d.update(path.read_bytes()); return d.hexdigest()

def main() -> int:
    if len(sys.argv) != 2: raise ValueError("usage: check-proof-phase5-boundary-final.py RECEIPT")
    receipt_path = Path(sys.argv[1]); subprocess.run([sys.executable, str(BASE), str(receipt_path)], cwd=ROOT, check=True)
    value = json.loads(receipt_path.read_text(encoding="utf-8"))
    if value.get("execution_profile") != "phase5-source-v2-to-datatype-v1": raise ValueError("wrong Phase 5 execution profile")
    by_path = {entry["path"]: entry["sha256"] for entry in value["files"]}
    missing = REQUIRED_PATHS - set(by_path)
    if missing: raise ValueError(f"missing execution boundary files: {sorted(missing)}")
    for relative in REQUIRED_PATHS:
        if by_path[relative] != sha(ROOT/relative): raise ValueError(f"execution boundary digest mismatch: {relative}")
    print(f"proof-phase5-boundary-final: PASS: files={len(value['files'])}"); return 0

if __name__ == "__main__":
    try: raise SystemExit(main())
    except (OSError, ValueError, subprocess.SubprocessError, json.JSONDecodeError) as exc:
        print(f"proof-phase5-boundary-final: FAIL: {exc}", file=sys.stderr); raise SystemExit(1)
