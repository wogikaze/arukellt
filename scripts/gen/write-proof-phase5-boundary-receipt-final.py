#!/usr/bin/env python3
"""Extend the Phase 5 boundary receipt with source schema and execution entrypoints."""
from __future__ import annotations
import argparse, hashlib, json, subprocess, sys
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / "scripts/gen/write-proof-phase5-boundary-receipt.py"
EXTRA = (
    "schemas/typed-corehir-v2.schema.json",
    "scripts/gen/convert-typed-corehir-v2.py",
    "scripts/gen/write-smt-vcs-v5.py",
    "scripts/gen/prepare-proof-phase5-toolchain.py",
    "scripts/check/check-proof-phase5-boundary-receipt.py",
    ".github/workflows/proof-phase5.yml",
)

def sha(path: Path) -> str:
    d = hashlib.sha256(); d.update(path.read_bytes()); return d.hexdigest()

def main() -> int:
    parser = argparse.ArgumentParser(); parser.add_argument("--output", type=Path, default=ROOT/".build/proof/proof-phase5-boundary-final.json"); args = parser.parse_args()
    subprocess.run([sys.executable, str(BASE), "--output", str(args.output)], cwd=ROOT, check=True)
    value = json.loads(args.output.read_text(encoding="utf-8")); present = {entry["path"] for entry in value["files"]}
    for relative in EXTRA:
        if relative in present: continue
        path = ROOT/relative
        if not path.is_file(): raise ValueError(f"Phase 5 execution boundary file missing: {relative}")
        value["files"].append({"path": relative, "sha256": sha(path)})
    value["execution_profile"] = "phase5-source-v2-to-datatype-v1"
    args.output.write_text(json.dumps(value, indent=2, sort_keys=True)+"\n", encoding="utf-8")
    print(f"proof-phase5-boundary-final: PASS: files={len(value['files'])} output={args.output}"); return 0

if __name__ == "__main__":
    try: raise SystemExit(main())
    except (OSError, ValueError, subprocess.SubprocessError, json.JSONDecodeError) as exc:
        print(f"proof-phase5-boundary-final: FAIL: {exc}", file=sys.stderr); raise SystemExit(1)
