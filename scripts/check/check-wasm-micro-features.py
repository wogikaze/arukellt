#!/usr/bin/env python3
from __future__ import annotations
import sys
from pathlib import Path
REPO_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO_ROOT / "scripts" / "check"))
from _gate_open_skip import skip_if_any_open
CHECKS = (("src/compiler/wasm/sections_branch_hint.ark","branch_hint"),("src/compiler/wasm/intrinsics/helpers_numeric_sign_extend.ark","extend8_s"),("src/compiler/wasm/sections_i31ref_hint.ark","i31ref"),("src/compiler/wasm/sections_name.ark","name"))

def main() -> int:
    if (s := skip_if_any_open(["064","067","070","115"], "check-wasm-micro-features")) is not None: return s
    for rel, needle in CHECKS:
        p = REPO_ROOT / rel
        if not p.is_file() or needle not in p.read_text(encoding="utf-8"):
            print(f"FAIL: {rel}", file=sys.stderr); return 1
    if "run:scalar/sign_extend.ark" not in (REPO_ROOT/"tests/fixtures/manifest.txt").read_text(encoding="utf-8"):
        print("FAIL: sign_extend fixture", file=sys.stderr); return 1
    print("check-wasm-micro-features: ok"); return 0
if __name__ == "__main__": raise SystemExit(main())
