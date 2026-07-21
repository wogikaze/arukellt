#!/usr/bin/env python3
"""Isolated stage-3 latency probe: host RSS samples + KEEP_CLOCK --time phases.

Usage (repo root, NO other selfhost compiles):

    python3 scripts/debug/latency_rss_phase_probe.py

Writes NDJSON to ``.cursor/debug-5db792.log`` (session 5db792).
"""
from __future__ import annotations

import json
import os
import re
import subprocess
import sys
import threading
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
LOG = ROOT / ".cursor/debug-5db792.log"
SESSION = "5db792"
PHASE_RE = re.compile(r"\[arukellt\]\s+([A-Za-z0-9_.]+):\s*([0-9]+(?:\.[0-9]+)?)ms")
CLOCK = ROOT / ".build/selfhost/arukellt-s2-clock.wasm"
RUNTIME = ROOT / ".build/selfhost/arukellt-s2-runtime.wasm"


# #region agent log
def _agent_log(
    hypothesis_id: str,
    location: str,
    message: str,
    data: dict,
    run_id: str = "isolated-probe",
) -> None:
    payload = {
        "sessionId": SESSION,
        "runId": run_id,
        "hypothesisId": hypothesis_id,
        "location": location,
        "message": message,
        "data": data,
        "timestamp": int(time.time() * 1000),
    }
    with LOG.open("a", encoding="utf-8") as handle:
        handle.write(json.dumps(payload, ensure_ascii=False) + "\n")


# #endregion


def _find_wasmtime() -> str:
    home = Path.home() / ".wasmtime/bin/wasmtime"
    return str(home) if home.is_file() else "wasmtime"


def _list_main_compiles() -> list[dict]:
    found: list[dict] = []
    for proc in Path("/proc").iterdir():
        if not proc.name.isdigit():
            continue
        try:
            cmd = (proc / "cmdline").read_bytes().replace(b"\x00", b" ").decode()
        except OSError:
            continue
        if "wasmtime" not in cmd or "main.ark" not in cmd:
            continue
        try:
            status = (proc / "status").read_text(encoding="utf-8")
            rss = int(re.search(r"VmRSS:\s*(\d+)", status).group(1)) / 1024.0
            uptime = float(Path("/proc/uptime").read_text(encoding="utf-8").split()[0])
            start = int((proc / "stat").read_text(encoding="utf-8").split()[21])
            hz = os.sysconf("SC_CLK_TCK")
            etime = uptime - start / hz
        except (OSError, AttributeError, ValueError):
            rss = -1.0
            etime = -1.0
        found.append(
            {
                "pid": int(proc.name),
                "etime_s": round(etime, 1),
                "rss_mb": round(rss, 1),
                "uses_clock": "clock" in cmd,
                "cmd_tail": cmd[-180:],
            }
        )
    return found


def _sample_rss(pid: int) -> dict | None:
    proc = Path(f"/proc/{pid}")
    if not proc.exists():
        return None
    try:
        status = (proc / "status").read_text(encoding="utf-8")
        rss = int(re.search(r"VmRSS:\s*(\d+)", status).group(1)) / 1024.0
        io_text = (proc / "io").read_text(encoding="utf-8")
        rchar = int(re.search(r"rchar:\s*(\d+)", io_text).group(1))
        wchar = int(re.search(r"wchar:\s*(\d+)", io_text).group(1))
        uptime = float(Path("/proc/uptime").read_text(encoding="utf-8").split()[0])
        start = int((proc / "stat").read_text(encoding="utf-8").split()[21])
        hz = os.sysconf("SC_CLK_TCK")
        return {
            "etime_s": round(uptime - start / hz, 1),
            "rss_mb": round(rss, 1),
            "rchar": rchar,
            "wchar": wchar,
        }
    except (OSError, AttributeError, ValueError):
        return None


def main() -> int:
    run_id = os.environ.get("ARUKELLT_DEBUG_RUN_ID", "isolated-probe")
    use_clock = os.environ.get("ARUKELLT_DEBUG_USE_CLOCK", "1") == "1"
    timeout_s = int(os.environ.get("ARUKELLT_DEBUG_COMPILE_TIMEOUT", "3600"))
    compiler = CLOCK if use_clock else RUNTIME
    out_rel = ".ark-debug/main-latency-probe.wasm"

    # #region agent log
    existing = _list_main_compiles()
    _agent_log(
        "C",
        "latency_probe:preflight",
        "concurrent_main_compiles",
        {"count": len(existing), "procs": existing},
        run_id=run_id,
    )
    # #endregion
    if existing:
        print("REFUSE: concurrent main.ark wasmtime processes:", existing, file=sys.stderr)
        # #region agent log
        _agent_log(
            "C",
            "latency_probe:refuse",
            "refused_concurrent",
            {"count": len(existing), "procs": existing},
            run_id=run_id,
        )
        # #endregion
        return 2
    if not compiler.is_file():
        print(f"missing compiler: {compiler}", file=sys.stderr)
        # #region agent log
        _agent_log("D", "latency_probe:preflight", "missing_compiler", {"path": str(compiler)}, run_id=run_id)
        # #endregion
        return 3

    out = ROOT / out_rel
    out.parent.mkdir(parents=True, exist_ok=True)
    if out.is_file():
        out.unlink()

    wasmtime = _find_wasmtime()
    cmd = [
        wasmtime,
        "run",
        "--wasm",
        "gc",
        "--wasm",
        "function-references",
        "-W",
        "memory64=y",
        "-W",
        "max-memory-size=17179869184",
        "--dir=.",
        str(compiler),
        "--",
        "compile",
        "src/compiler/main.ark",
        "--target",
        "wasm32-gc",
        "--wasi-version",
        "wasi-p2",
        "--time",
        "-o",
        out_rel,
    ]
    # #region agent log
    _agent_log(
        "A",
        "latency_probe:start",
        "compile_start",
        {
            "compiler": str(compiler.relative_to(ROOT)),
            "use_clock": use_clock,
            "timeout_s": timeout_s,
            "cmd_tail": " ".join(cmd[-12:]),
        },
        run_id=run_id,
    )
    # #endregion

    samples: list[dict] = []
    stop = threading.Event()

    def sampler(pid_holder: list[int]) -> None:
        while not stop.wait(15.0):
            if not pid_holder:
                continue
            sample = _sample_rss(pid_holder[0])
            if sample is None:
                continue
            sample["t_wall"] = round(time.perf_counter() - t0, 1)
            samples.append(sample)
            # #region agent log
            _agent_log("B", "latency_probe:sample", "rss_io_sample", sample, run_id=run_id)
            # #endregion

    pid_holder: list[int] = []
    t0 = time.perf_counter()
    thread = threading.Thread(target=sampler, args=(pid_holder,), daemon=True)
    thread.start()
    proc = subprocess.Popen(
        cmd,
        cwd=str(ROOT),
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    pid_holder.append(proc.pid)
    try:
        stdout, stderr = proc.communicate(timeout=timeout_s)
    except subprocess.TimeoutExpired:
        proc.kill()
        stdout, stderr = proc.communicate()
        # #region agent log
        _agent_log(
            "A",
            "latency_probe:timeout",
            "compile_timeout",
            {"timeout_s": timeout_s, "samples_n": len(samples), "last": samples[-1] if samples else None},
            run_id=run_id,
        )
        # #endregion
        stop.set()
        thread.join(timeout=2)
        return 124
    stop.set()
    thread.join(timeout=2)
    wall_ms = (time.perf_counter() - t0) * 1000.0
    text = (stderr or "") + "\n" + (stdout or "")
    phases = {m.group(1): float(m.group(2)) for m in PHASE_RE.finditer(text)}
    growth = None
    if len(samples) >= 2:
        drss = samples[-1]["rss_mb"] - samples[0]["rss_mb"]
        det = samples[-1]["etime_s"] - samples[0]["etime_s"]
        growth = {
            "drss_mb": round(drss, 1),
            "detime_s": round(det, 1),
            "rss_mb_per_min": round(drss / (det / 60.0), 1) if det > 0 else None,
            "dwchar": samples[-1]["wchar"] - samples[0]["wchar"],
        }
    # #region agent log
    _agent_log(
        "A",
        "latency_probe:end",
        "compile_end",
        {
            "rc": proc.returncode,
            "wall_ms": round(wall_ms, 1),
            "phases": phases,
            "out_ok": out.is_file(),
            "growth": growth,
            "sample_first": samples[0] if samples else None,
            "sample_last": samples[-1] if samples else None,
            "stderr_tail": (stderr or "")[-1200:],
        },
        run_id=run_id,
    )
    # #endregion
    print("rc", proc.returncode)
    print("wall_ms", round(wall_ms, 1))
    print("phases", json.dumps(phases, indent=2))
    print("growth", growth)
    print("out_ok", out.is_file())
    if proc.returncode != 0:
        print((stderr or "")[-1500:], file=sys.stderr)
    return 0 if proc.returncode == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())
