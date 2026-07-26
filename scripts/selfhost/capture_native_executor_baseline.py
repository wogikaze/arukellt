#!/usr/bin/env python3
"""Capture dated native-executor baseline receipts (Phase 0).

Does not overwrite prior receipts. Run from the recovery worktree with
ARUKELLT_BUILD_DIR set.
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
import time
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--arena-runs", type=int, default=3)
    parser.add_argument("--gc-runs", type=int, default=3)
    parser.add_argument(
        "--out-dir",
        type=Path,
        default=None,
        help="Defaults to $ARUKELLT_BUILD_DIR/selfhost/native/baselines/<stamp>",
    )
    args = parser.parse_args()

    root = Path(__file__).resolve().parents[2]
    build = Path(os.environ.get("ARUKELLT_BUILD_DIR", root / ".build-native-recovery"))
    native_dir = build / "selfhost" / "native"
    stamp = time.strftime("%Y%m%d-%H%M%S")
    out_dir = args.out_dir or (native_dir / "baselines" / stamp)
    out_dir.mkdir(parents=True, exist_ok=True)

    meta = {
        "stamp": stamp,
        "cwd": str(root),
        "build_dir": str(build),
        "git_head": subprocess.check_output(
            ["git", "rev-parse", "HEAD"], cwd=root, text=True
        ).strip(),
        "runs": [],
    }

    def one(label: str, extra_env: dict[str, str], allow_high_rss: bool) -> None:
        env = os.environ.copy()
        env.update(extra_env)
        env["ARUKELLT_BUILD_DIR"] = str(build)
        cmd = [
            sys.executable,
            "scripts/manager.py",
            "selfhost",
            "native-executor",
            "--build",
        ]
        if allow_high_rss:
            cmd.append("--allow-high-rss")
        print(f"=== {label} ===", flush=True)
        result = subprocess.run(cmd, cwd=root, env=env, check=False)
        src = native_dir / "native-executor-receipt.json"
        dst = out_dir / f"{label}.json"
        if src.is_file():
            shutil.copy2(src, dst)
            receipt = json.loads(src.read_text(encoding="utf-8"))
        else:
            receipt = {"missing_receipt": True}
        meta["runs"].append(
            {
                "label": label,
                "exit_code": result.returncode,
                "receipt": str(dst.relative_to(out_dir)),
                "warm_ms": receipt.get("executor_wall_time_ms"),
                "rss": receipt.get("executor_peak_rss_bytes"),
                "gc_mode": receipt.get("gc_mode"),
                "strict": receipt.get("strict_gate_passed"),
            }
        )
        (out_dir / "baseline-meta.json").write_text(
            json.dumps(meta, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )

    for index in range(1, args.arena_runs + 1):
        one(
            f"arena-{index}",
            {"ARUKELLT_NATIVE_GC": "0"},
            allow_high_rss=True,
        )
    for index in range(1, args.gc_runs + 1):
        one(
            f"gc-{index}",
            {"ARUKELLT_NATIVE_GC": "1"},
            allow_high_rss=False,
        )

    print(f"baselines written under {out_dir}", flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
