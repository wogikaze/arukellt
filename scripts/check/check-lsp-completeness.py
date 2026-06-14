#!/usr/bin/env python3
from __future__ import annotations
import sys
from pathlib import Path
REPO_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO_ROOT / "scripts" / "check"))
from _gate_open_skip import skip_if_any_open

def main() -> int:
    if (s := skip_if_any_open(["219"], "check-lsp-completeness")) is not None: return s
    t = (REPO_ROOT/"src/compiler/lsp/dispatch_features.ark").read_text(encoding="utf-8")
    for m in ("textDocument/signatureHelp","textDocument/documentHighlight","textDocument/foldingRange","textDocument/selectionRange"):
        if m not in t: print(f"FAIL: {m}", file=sys.stderr); return 1
    fx = REPO_ROOT/"tests/fixtures/selfhost"
    for n in ("lsp_folding_range.lsp-script","lsp_selection_range.lsp-script"):
        if not (fx/n).is_file(): print(f"FAIL: {n}", file=sys.stderr); return 1
    print("check-lsp-completeness: ok"); return 0
if __name__ == "__main__": raise SystemExit(main())
