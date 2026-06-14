#!/usr/bin/env python3
from __future__ import annotations
import sys
from pathlib import Path
REPO_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO_ROOT / "scripts" / "check"))
from _gate_open_skip import skip_if_any_open

def main() -> int:
    if (s := skip_if_any_open(["217"], "check-code-actions")) is not None: return s
    a = (REPO_ROOT/"src/compiler/lsp/feature_code_action.ark").read_text(encoding="utf-8")
    d = (REPO_ROOT/"src/compiler/lsp/dispatch_features.ark").read_text(encoding="utf-8")
    if "textDocument/codeAction" not in d: print("FAIL: dispatch", file=sys.stderr); return 1
    for n in ("quickfix","organize_imports","fix_all"):
        if n not in a: print(f"FAIL: {n}", file=sys.stderr); return 1
    fx = REPO_ROOT/"tests/fixtures/selfhost"
    for n in ("lsp_fix_all.lsp-script","lsp_organize_imports.lsp-script","lsp_completion_auto_import.lsp-script"):
        if not (fx/n).is_file(): print(f"FAIL: {n}", file=sys.stderr); return 1
    print("check-code-actions: ok"); return 0
if __name__ == "__main__": raise SystemExit(main())
