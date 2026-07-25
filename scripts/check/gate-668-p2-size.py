#!/usr/bin/env python3
"""Gate for #668 — P2 native component size / no P1 adapter blob.

Proves hello.component.wasm stays under a checked-in ceiling and does not
embed WASI P1 adapter markers. Savings vs the historical ~97KB adapter path
are recorded in docs/data/p2-native-component-size-baseline.toml.
"""

from __future__ import annotations

import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

_SCRIPTS_DIR = Path(__file__).resolve().parents[1]
if str(_SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(_SCRIPTS_DIR))

from lib.selfhost_s2 import gate_env, is_current_selfhost_wasm  # noqa: E402

REPO_ROOT = Path(__file__).resolve().parents[2]
BASELINE = REPO_ROOT / "docs" / "data" / "p2-native-component-size-baseline.toml"
FORBIDDEN = (
    b"wasi_snapshot_preview1",
    b"wasi_unstable",
)


def _load_baseline() -> dict[str, object]:
    text = BASELINE.read_text(encoding="utf-8")
    out: dict[str, object] = {}
    for key in (
        "fixture",
        "max_bytes",
        "adapter_reference_bytes",
        "min_savings_vs_adapter_bytes",
    ):
        match = re.search(rf"^{key}\s*=\s*(.+)$", text, re.M)
        if not match:
            raise SystemExit(f"missing {key} in {BASELINE}")
        raw = match.group(1).strip().strip('"')
        out[key] = int(raw) if key != "fixture" else raw
    return out


def _compile_component(fixture_rel: str, out: Path) -> tuple[int, str]:
    compiler = REPO_ROOT / "scripts" / "run" / "arukellt-selfhost.sh"
    if not compiler.is_file():
        return 2, "missing scripts/run/arukellt-selfhost.sh"
    try:
        out_arg = str(out.relative_to(REPO_ROOT))
    except ValueError:
        out_arg = str(out)
    result = subprocess.run(
        [
            "bash",
            str(compiler),
            "compile",
            fixture_rel,
            "--target",
            "wasm32-gc",
            "--wasi-version",
            "wasi-p2",
            "--emit",
            "component",
            "-o",
            out_arg,
        ],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=180,
    )
    if result.returncode != 0:
        return 1, (result.stderr or result.stdout)[-800:]
    if not out.is_file():
        return 1, f"missing output {out}"
    return 0, ""


def main() -> int:
    if not BASELINE.is_file():
        print(f"gate-668-p2-size: FAIL missing {BASELINE}", file=sys.stderr)
        return 1
    try:
        env = gate_env(REPO_ROOT, build=False)
    except Exception as exc:  # noqa: BLE001 — surface gate_env errors plainly
        print(f"gate-668-p2-size: FAIL selfhost env: {exc}", file=sys.stderr)
        return 1
    wasm = Path(env.get("ARUKELLT_SELFHOST_WASM", ""))
    if not is_current_selfhost_wasm(wasm):
        print(
            "gate-668-p2-size: FAIL proof must use current s2/s3 "
            f"(got {wasm})",
            file=sys.stderr,
        )
        return 1

    baseline = _load_baseline()
    fixture = str(baseline["fixture"])
    max_bytes = int(baseline["max_bytes"])
    adapter_ref = int(baseline["adapter_reference_bytes"])
    min_savings = int(baseline["min_savings_vs_adapter_bytes"])

    out_dir = Path(tempfile.mkdtemp(prefix="gate-668-size-", dir=REPO_ROOT / ".build"))
    failures: list[str] = []
    try:
        out = out_dir / "hello.component.wasm"
        rc, msg = _compile_component(fixture, out)
        if rc != 0:
            failures.append(f"compile: {msg}")
        else:
            size = out.stat().st_size
            if size > max_bytes:
                failures.append(f"size {size} > max_bytes {max_bytes}")
            savings = adapter_ref - size
            if savings < min_savings:
                failures.append(
                    f"savings vs adapter {savings} < min {min_savings} "
                    f"(size={size}, adapter_ref={adapter_ref})"
                )
            data = out.read_bytes()
            for marker in FORBIDDEN:
                if marker in data:
                    failures.append(f"forbidden marker {marker!r}")
            if b"@0.2.6" in data:
                failures.append("artifact contains @0.2.6 version string")
            if b"wasi:cli/run@0.2.0" not in data:
                failures.append("artifact missing wasi:cli/run@0.2.0")
    finally:
        shutil.rmtree(out_dir, ignore_errors=True)

    if failures:
        print("gate-668-p2-size: FAIL", file=sys.stderr)
        for line in failures:
            print(f"  - {line}", file=sys.stderr)
        return 1
    print("gate-668-p2-size: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
