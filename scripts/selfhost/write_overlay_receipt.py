#!/usr/bin/env python3
"""Parse --time / [arukellt] overlay stderr into a machine-readable receipt."""
from __future__ import annotations

import argparse
import json
import re
from datetime import datetime, timezone
from pathlib import Path

PHASE_RE = re.compile(r"\[arukellt\]\s+([^:]+):\s+(\d+)ms")
SLOW_FN_RE = re.compile(
    r"\[arukellt\]\s+emit\.slow_fn:\s+idx=(\d+)\s+insts=(\d+)\s+ms=(\d+)\s+(\S+)"
)
SCHEMA = "arukellt-selfhost-overlay-receipt-v1"


def parse_stderr(text: str) -> tuple[dict[str, int], list[dict[str, object]]]:
    phases: dict[str, int] = {}
    slow_fns: list[dict[str, object]] = []
    for raw in text.splitlines():
        phase_match = PHASE_RE.search(raw)
        if phase_match and "emit.slow_fn" not in raw:
            phases[phase_match.group(1).strip()] = int(phase_match.group(2))
        slow_match = SLOW_FN_RE.search(raw)
        if slow_match:
            slow_fns.append(
                {
                    "name": slow_match.group(4),
                    "insts": int(slow_match.group(2)),
                    "wall_ms": int(slow_match.group(3)),
                }
            )
    return phases, slow_fns[:32]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--stderr", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--host", default="")
    parser.add_argument("--target", default="wasm32-gc")
    parser.add_argument("--wasi-version", default="wasi-p2")
    parser.add_argument("--wall-ms", type=int, default=0)
    parser.add_argument("--rss-max-kb", type=int, default=0)
    parser.add_argument("--s2-sha256", default="")
    parser.add_argument("--s3-sha256", default="")
    parser.add_argument("--s3-valid", action="store_true")
    parser.add_argument("--notes", default="")
    args = parser.parse_args()

    phases, slow_fns = parse_stderr(args.stderr.read_text(encoding="utf-8", errors="replace"))
    if not phases:
        raise SystemExit("no [arukellt] phase lines; clock stub or discarded stderr")

    receipt = {
        "schema": SCHEMA,
        "recorded_at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "host": args.host,
        "target": args.target,
        "wasi_version": args.wasi_version,
        "wall_ms": args.wall_ms,
        "rss_max_kb": args.rss_max_kb,
        "s2_sha256": args.s2_sha256,
        "s3_sha256": args.s3_sha256,
        "s2_eq_s3": bool(args.s2_sha256) and args.s2_sha256 == args.s3_sha256,
        "s3_valid": args.s3_valid,
        "alloc_bytes": None,
        "live_bytes_by_phase": {},
        "mir_counts": {},
        "slow_fns": slow_fns,
        "phases_ms": phases,
        "notes": args.notes,
    }
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {args.out} phases={len(phases)} slow_fns={len(slow_fns)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
