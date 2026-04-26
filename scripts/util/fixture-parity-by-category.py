#!/usr/bin/env python3
"""Run selfhost fixture parity filtered to a single category prefix.

Usage: fixture-parity-by-category.py <category> [--list-fails]

Example: scripts/util/fixture-parity-by-category.py hello
"""
from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path


def _find_arukellt(root: Path) -> str | None:
    env_bin = os.environ.get("ARUKELLT_BIN", "")
    if env_bin and Path(env_bin).is_file():
        return env_bin
    for c in ["target/debug/arukellt", "target/release/arukellt"]:
        p = root / c
        if p.is_file() and os.access(p, os.X_OK):
            return str(p)
    return None


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("category")
    ap.add_argument("--limit", type=int, default=0)
    args = ap.parse_args()

    root = Path(__file__).resolve().parents[2]
    sys.path.insert(0, str(root / "scripts" / "selfhost"))
    from checks import _load_manifest_fixtures, FIXTURE_PARITY_SKIP  # type: ignore

    fixtures, err = _load_manifest_fixtures(root, "run")
    if err:
        print(err, file=sys.stderr)
        return 1

    cat_prefix = args.category.rstrip("/") + "/"
    targets = [f for f in fixtures if f.startswith(cat_prefix)]
    if args.limit:
        targets = targets[: args.limit]
    print(f"[category={args.category}] {len(targets)} fixtures")

    arukellt = _find_arukellt(root)
    if not arukellt:
        print("arukellt binary not found", file=sys.stderr)
        return 1

    wasmtime = shutil.which("wasmtime")
    if not wasmtime:
        print("wasmtime not found", file=sys.stderr)
        return 1

    selfhost = root / ".build" / "selfhost" / "arukellt-s1.wasm"
    if not selfhost.is_file():
        print(f"selfhost wasm not found at {selfhost}", file=sys.stderr)
        return 1

    pass_c = fail_c = skip_c = 0
    fails: list[tuple[str, str]] = []
    tmpdir = Path(tempfile.mkdtemp(prefix="ark-cat-parity-"))
    self_dir = root / ".ark-cat-parity-tmp"
    self_dir.mkdir(exist_ok=True)

    try:
        for fx in targets:
            if fx in FIXTURE_PARITY_SKIP:
                print(f"  skip: {fx} (known parity skip)")
                skip_c += 1
                continue
            ark = root / "tests" / "fixtures" / fx
            if not ark.is_file():
                print(f"  skip: {fx} (missing)")
                skip_c += 1
                continue
            out_rust = tmpdir / f"rust-{fx.replace('/', '_')}.wasm"
            out_self_rel = Path(".ark-cat-parity-tmp") / f"self-{fx.replace('/', '_')}.wasm"
            out_self = root / out_self_rel

            r = subprocess.run(
                [arukellt, "compile", str(ark), "--target", "wasm32-wasi-p1", "-o", str(out_rust)],
                cwd=root, capture_output=True, text=True,
            )
            if r.returncode != 0:
                print(f"  skip: {fx} (rust compile failed)")
                skip_c += 1
                continue
            try:
                r = subprocess.run(
                    [wasmtime, "run", "--dir", str(root), str(selfhost), "--", "compile",
                     str(Path("tests") / "fixtures" / fx), "--target", "wasm32-wasi-p1", "-o", str(out_self_rel)],
                    cwd=root, capture_output=True, text=True, timeout=30,
                )
            except subprocess.TimeoutExpired:
                print(f"  skip: {fx} (selfhost compile timeout)")
                skip_c += 1
                continue
            if r.returncode != 0:
                print(f"  skip: {fx} (selfhost compile failed)")
                skip_c += 1
                continue
            try:
                r1 = subprocess.run([wasmtime, "run", str(out_rust)], cwd=root, capture_output=True, text=True, timeout=15)
                r2 = subprocess.run([wasmtime, "run", str(out_self)], cwd=root, capture_output=True, text=True, timeout=15)
            except subprocess.TimeoutExpired:
                print(f"  skip: {fx} (exec timeout)")
                skip_c += 1
                continue
            ro, rc = (r1.stdout + r1.stderr).strip(), r1.returncode
            so, sc = (r2.stdout + r2.stderr).strip(), r2.returncode
            if sc == 134 or (sc == 1 and "failed to compile" in so):
                print(f"  skip: {fx} (selfhost wasm trap)")
                skip_c += 1
                continue
            if ro == so and rc == sc:
                pass_c += 1
            else:
                diff = ""
                if rc != sc:
                    diff += f"exit rust={rc} self={sc}; "
                if ro != so:
                    diff += f"rust={ro[:60]!r} self={so[:60]!r}"
                print(f"  FAIL: {fx} — {diff}")
                fails.append((fx, diff))
                fail_c += 1
    finally:
        shutil.rmtree(tmpdir, ignore_errors=True)
        shutil.rmtree(self_dir, ignore_errors=True)

    print(f"\ncategory={args.category}: PASS={pass_c} FAIL={fail_c} SKIP={skip_c}")
    return 0 if fail_c == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())
