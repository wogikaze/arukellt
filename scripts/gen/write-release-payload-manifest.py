#!/usr/bin/env python3
"""Write a hash-bound manifest for final release payloads."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from proof.release_payload import create_release_payload_manifest  # noqa: E402


def parse_payload(raw: str) -> tuple[str, Path]:
    name, separator, path = raw.partition("=")
    if not separator or not name or not path:
        raise ValueError("--payload must be NAME=PATH")
    return name, Path(path)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--payload", action="append", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    payloads: dict[str, Path] = {}
    for raw in args.payload:
        name, path = parse_payload(raw)
        if name in payloads:
            raise ValueError(f"duplicate payload name: {name}")
        payloads[name] = path
    document = create_release_payload_manifest(payloads)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(document, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(
        "release-payload-manifest: PASS: "
        f"artifacts={len(document['artifacts'])} output={args.output}"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError) as exc:
        print(f"release-payload-manifest: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
