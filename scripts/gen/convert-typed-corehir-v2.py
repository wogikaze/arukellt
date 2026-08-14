#!/usr/bin/env python3
"""Convert proof-facing TypedCoreHIR v1/v2 to VerifiedCore through Phase 5."""
from __future__ import annotations
import argparse, json, sys
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]; SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path: sys.path.insert(0, str(SCRIPTS))
from proof.typed_corehir_v2_convert_final import convert_document

def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__); parser.add_argument("--input", type=Path, required=True); parser.add_argument("--output", type=Path, required=True); args = parser.parse_args()
    value = json.loads(args.input.read_text(encoding="utf-8")); converted = convert_document(value)
    args.output.parent.mkdir(parents=True, exist_ok=True); args.output.write_text(json.dumps(converted, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"typed-corehir-v2-convert: PASS: functions={len(converted['functions'])} output={args.output}"); return 0

if __name__ == "__main__":
    try: raise SystemExit(main())
    except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError) as exc:
        print(f"typed-corehir-v2-convert: FAIL: {exc}", file=sys.stderr); raise SystemExit(1)
