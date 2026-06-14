#!/usr/bin/env python3
from __future__ import annotations
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]


def main() -> int:
    gate = REPO_ROOT / "src/compiler/resolver/host_stub_gate.ark"
    text = gate.read_text(encoding="utf-8")
    if "host_stub" not in text or "E0500" not in text or "udp::send" not in text:
        print("FAIL: host_stub_gate", file=sys.stderr)
        return 1
    manifest = (REPO_ROOT / "tests/fixtures/manifest.txt").read_text(encoding="utf-8")
    if "diag:host_stub_sockets.ark" not in manifest:
        print("FAIL: manifest", file=sys.stderr)
        return 1
    diag = (REPO_ROOT / "tests/fixtures/host_stub_sockets.diag").read_text(encoding="utf-8")
    if "host_stub" not in diag:
        print("FAIL: diag", file=sys.stderr)
        return 1
    print("check-host-stub-gate: ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
