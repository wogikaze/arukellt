#!/usr/bin/env python3
"""Page-map membership: mark must not require a full pre-mark table rebuild."""

from __future__ import annotations

import json
import os
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BUILD = Path(os.environ.get("ARUKELLT_BUILD_DIR", str(ROOT / ".build-native-recovery")))
NATIVE = BUILD / "selfhost" / "native" / "arukellt-native"
FIXTURE = ROOT / "tests" / "fixtures" / "native_gc_stress" / "dead_reclaim.ark"


def test_native_gc_page_map_skips_full_table_rebuild() -> None:
    assert NATIVE.is_file(), f"missing native executor binary: {NATIVE}"
    assert FIXTURE.is_file(), f"missing fixture: {FIXTURE}"

    with tempfile.TemporaryDirectory(prefix="native-gc-page-map-") as tmp:
        tmp_path = Path(tmp)
        wasm_out = tmp_path / "dead_reclaim.wasm"
        stats_path = tmp_path / "gc.json"
        env = os.environ.copy()
        env["ARUKELLT_NATIVE_GC"] = "1"
        env["ARUKELLT_NATIVE_GC_THRESHOLD_BYTES"] = "65536"
        env["ARUKELLT_NATIVE_GC_STATS_PATH"] = str(stats_path)
        env["ARUKELLT_NATIVE_GC_VERIFY_PAGE_MAP"] = "1"
        proc = subprocess.run(
            [
                str(NATIVE),
                "compile",
                str(FIXTURE),
                "--target",
                "wasm32",
                "--wasi-version",
                "wasi-p1",
                "--output",
                str(wasm_out),
            ],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=False,
            env=env,
        )
        assert proc.returncode == 0, proc.stderr[-2000:] + proc.stdout[-2000:]
        assert "page-map miss=" not in proc.stderr
        stats = json.loads(stats_path.read_text())
        assert int(stats.get("gc_collection_count", 0)) > 0
        assert int(stats.get("gc_total_marked_objects", 0)) > 0
        rebuild = int(stats.get("gc_total_table_rebuild_time_ms", 0))
        mark = int(stats.get("gc_total_mark_time_ms", 0))
        sweep = int(stats.get("gc_total_sweep_time_ms", 0))
        # With page-map hits, emergency rebuild must not dominate collect.
        assert rebuild <= mark + sweep
        # Object-table capacity stays small (or zero) when page-map is SSOT.
        assert int(stats.get("gc_object_table_capacity", 0)) <= 1024


if __name__ == "__main__":
    test_native_gc_page_map_skips_full_table_rebuild()
    print("ok")
