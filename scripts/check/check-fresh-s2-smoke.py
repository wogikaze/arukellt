#!/usr/bin/env python3
"""Fresh-s2 producer smoke (#911).

Every day-to-day lane resolves the pinned bootstrap wasm first
(scripts/run/arukellt-selfhost.sh), so a regression that makes a freshly
built s2 hang or mis-emit as a producer ships undetected until the rare
fixpoint build (f678e96d8 left any `use std::seq` compile hanging
indefinitely under fresh s2).

This check compiles a bounded probe set directly through
.build/selfhost/arukellt-s2-runtime.wasm with a hard per-probe timeout,
then validates each emitted module with wasm-tools. A producer hang
trips the timeout; invalid emit trips validation; a diagnostic failure
fails the compile.
"""

from __future__ import annotations

import os
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
WRAPPER = REPO_ROOT / "scripts" / "run" / "arukellt-selfhost.sh"
S2_RUNTIME = REPO_ROOT / ".build" / "selfhost" / "arukellt-s2-runtime.wasm"
OUT_DIR = REPO_ROOT / ".build" / "s2-smoke"

# Hard bound per probe. Healthy probes finish in tens of seconds; the
# regression class this guards is an unbounded producer hang, so the
# timeout only needs to separate "slow but finite" from "never
# finishes".
PROBE_TIMEOUT_SECONDS = 420

# Probes cover the producer failure modes: generic monomorphization at
# multiple instantiations, emit of both wasm32-gc and wasm32 outputs,
# and run-time correctness of emitted modules. std::seq generic-wrappers
# are deliberately out of the probe set for now: under the un-repinned
# s2 (old linear-GC collect baked in) a std::seq import alone pushes the
# producer into the quadratic mark path for minutes, which would leave
# the smoke permanently red until the runtime is re-pinned. The
# dual-instantiation probes below still exercise the mono instance
# mint/bind path that the historical hang and mis-bind regressions hit.
#
# The linear inline probe keeps `use std::test` asserts so the emitted
# module can run standalone and catch silent mis-binds at run time. Do
# not "tidy" it into the gc probe's stdio shape: std::test transitively
# pulls in `std::core::identity(x: i32)`, which collides with the local
# generic `identity` on the gc emit path today (issue #914) - the same
# program emitted for wasm32-gc fails validation. The gc dual probe
# therefore uses std::host::stdio instead, which does not import the
# colliding symbol.
INLINE_PROBE_LINEAR_SOURCE = """use std::test

fn identity<T>(x: T) -> T {
    x
}

fn main() {
    let a = identity(7)
    let b = identity(String_from("ok"))
    test::assert_eq_i32(a, 7)
    test::assert_eq_string(b, String_from("ok"))
}
"""

INLINE_PROBE_GC_SOURCE = """use std::host::stdio

fn identity<T>(x: T) -> T {
    x
}

fn main() {
    let a = identity(7)
    let b = identity(String_from("ok"))
    stdio::println(b)
}
"""

PROBES = [
    (
        "generics-two-params-gc",
        "tests/fixtures/generics/two_params.ark",
        "wasm32-gc",
    ),
    (
        "generics-dual-inline-linear",
        None,
        "wasm32",
    ),
    (
        "generics-dual-inline-gc",
        None,
        "wasm32-gc",
    ),
    (
        "generics-vec-gc",
        "tests/fixtures/generics/vec_generic.ark",
        "wasm32-gc",
    ),
]

INLINE_PROBE_SOURCES = {
    "generics-dual-inline-linear": INLINE_PROBE_LINEAR_SOURCE,
    "generics-dual-inline-gc": INLINE_PROBE_GC_SOURCE,
}


# Probes that additionally execute the emitted module; a non-zero exit
# catches silent miscompiles that pass validation (wrong-instance binds).
# Only assert-only programs can run standalone: stdio::println imports a
# WASI host capability that plain wasmtime run does not provide.
RUN_PROBES = {"generics-dual-inline-linear"}


def _run_probe(name: str, fixture: str | None, target: str) -> tuple[str, bool, str]:
    out = OUT_DIR / f"{name}.wasm"
    if fixture is None:
        probe_src = OUT_DIR / f"{name}.ark"
        probe_src.write_text(INLINE_PROBE_SOURCES[name], encoding="utf-8")
        fixture = str(probe_src.relative_to(REPO_ROOT))
    env = dict(os.environ)
    env["ARUKELLT_SELFHOST_WASM"] = str(S2_RUNTIME)
    start = time.monotonic()
    try:
        result = subprocess.run(
            [
                str(WRAPPER),
                "compile",
                fixture,
                "--target",
                target,
                "-o",
                str(out.relative_to(REPO_ROOT)),
            ],
            cwd=str(REPO_ROOT),
            capture_output=True,
            text=True,
            timeout=PROBE_TIMEOUT_SECONDS,
            env=env,
        )
    except subprocess.TimeoutExpired:
        return (
            name,
            False,
            f"producer timeout after {PROBE_TIMEOUT_SECONDS}s "
            f"({fixture} --target {target})",
        )
    elapsed = time.monotonic() - start
    if result.returncode != 0:
        tail = (result.stdout + result.stderr).strip().splitlines()[-5:]
        return (
            name,
            False,
            f"compile exited {result.returncode} in {elapsed:.0f}s: "
            + " | ".join(tail),
        )
    if not out.is_file():
        return name, False, f"compile reported success but {out} missing"
    validate = subprocess.run(
        ["wasm-tools", "validate", str(out)],
        capture_output=True,
        text=True,
    )
    if validate.returncode != 0:
        tail = (validate.stdout + validate.stderr).strip().splitlines()[-5:]
        return (
            name,
            False,
            f"emitted module failed wasm-tools validate: "
            + " | ".join(tail),
        )
    if name in RUN_PROBES:
        try:
            run = subprocess.run(
                [
                    "wasmtime",
                    "run",
                    "--wasm",
                    "gc",
                    "--wasm",
                    "function-references",
                    "-W",
                    "memory64=y",
                    str(out),
                ],
                capture_output=True,
                text=True,
                timeout=120,
            )
        except subprocess.TimeoutExpired:
            return name, False, "emitted module timed out at run time"
        if run.returncode != 0:
            tail = (run.stdout + run.stderr).strip().splitlines()[-5:]
            return (
                name,
                False,
                f"emitted module exited {run.returncode} at run time: "
                + " | ".join(tail),
            )
    return name, True, f"ok in {elapsed:.0f}s"


def main() -> int:
    if not S2_RUNTIME.is_file():
        print(
            "SKIP: .build/selfhost/arukellt-s2-runtime.wasm missing "
            "(run scripts/manager.py selfhost build-compiler)",
            file=sys.stderr,
        )
        return 0
    if not WRAPPER.is_file() or not os.access(WRAPPER, os.X_OK):
        print("SKIP: selfhost wrapper missing", file=sys.stderr)
        return 0
    if subprocess.run(["wasm-tools", "--version"], capture_output=True).returncode != 0:
        print("SKIP: wasm-tools missing", file=sys.stderr)
        return 0

    OUT_DIR.mkdir(parents=True, exist_ok=True)
    failures: list[str] = []
    with ThreadPoolExecutor(max_workers=len(PROBES)) as pool:
        futures = [
            pool.submit(_run_probe, name, fixture, target)
            for name, fixture, target in PROBES
        ]
        for future in futures:
            name, ok, detail = future.result()
            status = "PASS" if ok else "FAIL"
            print(f"{status}: {name}: {detail}")
            if not ok:
                failures.append(name)

    if failures:
        print(
            f"FAIL: fresh-s2 producer smoke failed: {', '.join(failures)}",
            file=sys.stderr,
        )
        return 1
    print(f"fresh-s2 producer smoke: {len(PROBES)} probes passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
