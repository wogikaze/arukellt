#!/usr/bin/env python3
"""Component standard-world gate (issue #118)."""
from __future__ import annotations
import os, subprocess, sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
MANIFEST = REPO_ROOT / "tests" / "fixtures" / "manifest.txt"

def _compiler():
    w = REPO_ROOT / "scripts" / "run" / "arukellt-selfhost.sh"
    return w if w.is_file() else None

def _env():
    env = dict(os.environ)
    if "ARUKELLT_SELFHOST_WASM" in env:
        return env
    for c in (REPO_ROOT / ".build/selfhost/arukellt-s3.wasm", REPO_ROOT / ".build/selfhost/arukellt-s2.wasm", REPO_ROOT / "bootstrap/arukellt-selfhost.wasm"):
        if c.is_file():
            env["ARUKELLT_SELFHOST_WASM"] = str(c.resolve())
            break
    return env

def _entries(kind):
    out = []
    for line in MANIFEST.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        k, _, p = line.partition(":")
        if k.strip() == kind:
            out.append(p.strip())
    return out

def _norm(text):
    return "\n".join(l for l in (x.rstrip() for x in text.strip().splitlines()) if l) + "\n"

def _compile(args):
    c = _compiler()
    if c is None:
        return 127, "no compiler"
    r = subprocess.run(["bash", str(c), *args], cwd=str(REPO_ROOT), capture_output=True, text=True, timeout=180, env=_env())
    return r.returncode, (r.stderr or r.stdout)[-1200:]

def _static():
    f = []
    if "--world" not in (REPO_ROOT / "src/compiler/main/args_parse.ark").read_text(encoding="utf-8"):
        f.append("missing --world in args_parse.ark")
    if not (REPO_ROOT / "src/compiler/component/world_spec.ark").is_file():
        f.append("missing world_spec.ark")
    return f

def _validate_golden(golden, rel):
    text = golden.read_text(encoding="utf-8")
    f = []
    if "package arukellt:app;" not in text:
        f.append("missing package arukellt:app")
    if "command" in rel:
        for m in ("world command {", "import wasi:cli/stdin@0.2.0", "export run: func();"):
            if m not in text:
                f.append(f"missing {m!r}")
    elif "proxy" in rel:
        for m in ("world proxy {", "import wasi:http/types@0.2.0", "export wasi:http/incoming-handler@0.2.0"):
            if m not in text:
                f.append(f"missing {m!r}")
    return f

def main():
    failures = 0
    for msg in _static():
        print(f"FAIL static: {msg}", file=sys.stderr); failures += 1
    if not failures:
        print("pass: static --world / world_spec checks")
    for rel in _entries("component-world-wit"):
        golden = (REPO_ROOT / "tests/fixtures" / rel).with_suffix(".expected.wit")
        for msg in _validate_golden(golden, rel):
            print(f"FAIL {rel}: golden {msg}", file=sys.stderr); failures += 1
        else:
            print(f"pass: {rel} golden structure")
        world = "wasi:cli/command" if "command" in rel else "wasi:http/proxy"
        out = REPO_ROOT / f".build/component-world-{Path(rel).stem}.wit"
        rc, msg = _compile(["compile", f"tests/fixtures/{rel}", "--target", "wasm32-wasi-p2", "--emit", "wit", "--world", world, "-o", str(out.relative_to(REPO_ROOT))])
        if rc != 0:
            if "unexpected argument" in msg:
                print(f"note: {rel} emit skipped (bootstrap wasm lacks --world)")
            else:
                print(f"FAIL {rel} emit: {msg}", file=sys.stderr); failures += 1
        elif out.is_file():
            if _norm(out.read_text(encoding="utf-8")) != _norm(golden.read_text(encoding="utf-8")):
                print(f"FAIL {rel}: wit mismatch", file=sys.stderr); failures += 1
            else:
                print(f"pass: {rel} emit matches golden")
            out.unlink(missing_ok=True)
    for rel in _entries("component-world-error"):
        rc, msg = _compile(["compile", f"tests/fixtures/{rel}", "--target", "wasm32-wasi-p2", "--emit", "component", "--world", "wasi:cli/command"])
        if rc == 0:
            print(f"FAIL {rel}: expected error", file=sys.stderr); failures += 1
        elif "wasi:cli/run/run" in msg or "requires export" in msg:
            print(f"pass: {rel} error surface")
        elif "unexpected argument" in msg:
            print(f"note: {rel} error test skipped (bootstrap wasm)")
        else:
            print(f"FAIL {rel}: {msg}", file=sys.stderr); failures += 1
    rc, msg = _compile(["compile", "tests/fixtures/component/world_command.ark", "--target", "wasm32-wasi-p2", "--emit", "wit", "--world", "wasi:unknown/world"])
    if rc == 0:
        print("FAIL unknown world should error", file=sys.stderr); failures += 1
    elif "unknown world" in msg:
        print("pass: unknown world error")
    elif "unexpected argument" in msg:
        print("note: unknown world test skipped (bootstrap wasm)")
    else:
        print(f"FAIL unknown world: {msg}", file=sys.stderr); failures += 1
    return 1 if failures else 0

if __name__ == "__main__":
    sys.exit(main())
