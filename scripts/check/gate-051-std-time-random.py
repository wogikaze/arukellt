#!/usr/bin/env python3
"""Close gate for issue #051 — std::time + std::random umbrella."""

from __future__ import annotations

import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]

MANIFEST_ENTRIES = (
    "t3-run:stdlib_time/monotonic.ark",
    "module-run:stdlib_time/duration.ark",
    "t3-run:stdlib_host/wasi_clock.ark",
    "module-run:stdlib_random/random_basic.ark",
    "module-run:stdlib_random/random_range.ark",
    "module-run:stdlib_random/random_seed.ark",
    "module-run:stdlib_random/shuffle.ark",
)

REQUIRED_PATHS = (
    "std/time/mod.ark",
    "std/host/clock.ark",
    "std/random/mod.ark",
    "src/compiler/wasm/intrinsic_clock.ark",
    "src/compiler/wasm/call_host_time.ark",
    "tests/fixtures/stdlib_time/monotonic.ark",
    "tests/fixtures/stdlib_time/duration.ark",
    "tests/fixtures/stdlib_host/wasi_clock.ark",
)


def main() -> int:
    failures: list[str] = []

    manifest_path = REPO_ROOT / "tests" / "fixtures" / "manifest.txt"
    if not manifest_path.is_file():
        failures.append("missing tests/fixtures/manifest.txt")
        manifest = ""
    else:
        manifest = manifest_path.read_text(encoding="utf-8")

    for entry in MANIFEST_ENTRIES:
        if entry not in manifest:
            failures.append(f"manifest missing {entry}")

    for rel in REQUIRED_PATHS:
        if not (REPO_ROOT / rel).is_file():
            failures.append(f"missing {rel}")

    clock_emitter = (REPO_ROOT / "src/compiler/wasm/call_host_time.ark").read_text(
        encoding="utf-8"
    )
    for intrinsic in ("__intrinsic_clock_now", "__intrinsic_clock_now_ms"):
        if intrinsic not in clock_emitter:
            failures.append(f"call_host_time.ark lacks {intrinsic} dispatch")

    random_emitter = (REPO_ROOT / "src/compiler/wasm").glob("*.ark")
    random_found = False
    for path in random_emitter:
        text = path.read_text(encoding="utf-8", errors="replace")
        if "__intrinsic_random_i32" in text:
            random_found = True
            break
    if not random_found:
        failures.append("no wasm emitter dispatch for __intrinsic_random_i32")

    if failures:
        print("gate-051-std-time-random: FAIL", file=sys.stderr)
        for line in failures:
            print(f"  - {line}", file=sys.stderr)
        return 1

    print("gate-051-std-time-random: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
