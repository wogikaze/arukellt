#!/usr/bin/env python3
"""Cross-backend semantic parity audit for issue #875.

Compiles each selected run: fixture twice with the selfhost compiler -
once as a wasm32-gc WASI-P2 component (primary path) and once as a
wasm32 linear-memory core module (fallback path) - runs both under
wasmtime, and compares normalized stdout so semantic parity is proven by
observation rather than inferred from a green single-target gate.

Outputs a JSON receipt with per-fixture results, artifact SHA-256 hashes,
and the runtime/toolchain versions required by the #875 acceptance list.
"""

from __future__ import annotations

import argparse
import concurrent.futures
import hashlib
import json
import os
import re
import subprocess
import sys
import time
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
MANIFEST = ROOT / "tests" / "fixtures" / "manifest.txt"
FIXTURES_ROOT = ROOT / "tests" / "fixtures"
SELFHOST_RUNNER = ROOT / "scripts" / "run" / "arukellt-selfhost.sh"
OUT_DIR_REL = ".build/parity"

# Semantics-bearing fixture domains mapped to RFC-011 items for the #875
# audit matrix. stdlib_*/selfhost/native_* lanes are excluded: they either
# run on the same two emitters with no additional semantics coverage per
# fixture, or carry their own target-specific contracts.
DEFAULT_DOMAINS = (
    "arrays/",
    "associated_fn/",
    "closure_capture/",
    "collections/",
    "control/",
    "enums/",
    "f32_basic/",
    "for_loops/",
    "from_trait/",
    "functions/",
    "generics/",
    "generics_v1/",
    "iterator/",
    "lang_uplift/",
    "match_extensions/",
    "module_import/",
    "operator_overload/",
    "operators/",
    "ownership/",
    "question_mark/",
    "scalar/",
    "string_interp/",
    "struct_mutation/",
    "structs/",
    "trait/",
    "v0_constraints/",
    "variables/",
)


def _manifest_run_fixtures() -> list[str]:
    fixtures: list[str] = []
    for line in MANIFEST.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if line.startswith("run:"):
            fixtures.append(line[4:])
    return fixtures


def _sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _normalize_output(text: str) -> str:
    bt = chr(96)
    text = re.sub(bt + "[^" + bt + "]*" + bt, bt + "out" + bt, text)
    text = re.sub(r"(?<![0-9a-zA-Z.])0x[0-9a-fA-F]+", "0x<addr>", text)
    text = re.sub(
        r"(?:^|\n)Error: failed to run main module.*", "", text, flags=re.DOTALL
    )
    return text


def _run(argv: list[str], timeout: int = 60) -> subprocess.CompletedProcess:
    return subprocess.run(
        argv,
        cwd=ROOT,
        capture_output=True,
        text=True,
        timeout=timeout,
        env={**os.environ},
    )


def _compile(target: str, emit: str | None, fixture: str, out_rel: str) -> tuple[bool, str]:
    argv = [
        "bash",
        str(SELFHOST_RUNNER.relative_to(ROOT)),
        "compile",
        "--target",
        target,
    ]
    if emit:
        argv += ["--emit", emit]
    argv += ["-o", out_rel, fixture]
    proc = _run(argv, timeout=120)
    ok = proc.returncode == 0 and "compilation succeeded" in (
        proc.stdout + proc.stderr
    )
    detail = "" if ok else (proc.stderr + proc.stdout)[-400:]
    return ok, detail


def _execute(argv: list[str]) -> tuple[int, str]:
    proc = _run(argv)
    return proc.returncode, _normalize_output(proc.stdout + proc.stderr)


def _runtime_dirs(fixture: str) -> list[str]:
    # Mirror _fixture_runtime_args: run fixtures default to repo-root preopen;
    # --deny-fs sidecars mean no --dir at all.
    flags_path = FIXTURES_ROOT / (fixture[:-4] + ".flags")
    if flags_path.is_file() and "--deny-fs" in flags_path.read_text(
        encoding="utf-8"
    ):
        return []
    return ["--dir", str(ROOT)]


def _probe_fixture(fixture: str, out_dir: Path) -> dict:
    slug = fixture.replace("/", "_")[:-4]
    gc_comp_rel = f"{OUT_DIR_REL}/{slug}.gc.component.wasm"
    lm_core_rel = f"{OUT_DIR_REL}/{slug}.w32.wasm"
    record: dict = {"fixture": fixture, "status": "unknown"}
    source = f"tests/fixtures/{fixture}"

    gc_ok, gc_detail = _compile(
        "wasm32-gc", "component", source, gc_comp_rel
    )
    if not gc_ok:
        record.update({"status": "compile-fail:gc", "detail": gc_detail})
        return record
    lm_ok, lm_detail = _compile("wasm32", None, source, lm_core_rel)
    if not lm_ok:
        record.update({"status": "compile-fail:wasm32", "detail": lm_detail})
        return record

    gc_path = ROOT / gc_comp_rel
    lm_path = ROOT / lm_core_rel
    record["gc_component_sha256"] = _sha256(gc_path)
    record["lm_core_sha256"] = _sha256(lm_path)
    record["gc_component_bytes"] = gc_path.stat().st_size
    record["lm_core_bytes"] = lm_path.stat().st_size

    dirs = _runtime_dirs(fixture)
    gc_rc, gc_out = _execute(
        [
            "wasmtime",
            "run",
            "--wasm",
            "gc",
            "--wasm",
            "function-references",
            *dirs,
            gc_comp_rel,
        ]
    )
    lm_rc, lm_out = _execute(
        [
            "wasmtime",
            "run",
            "--wasm",
            "gc",
            "--wasm",
            "function-references",
            "-W",
            "memory64=y",
            *dirs,
            lm_core_rel,
        ]
    )
    record["gc_rc"] = gc_rc
    record["lm_rc"] = lm_rc

    expected_path = FIXTURES_ROOT / (fixture[:-4] + ".expected")
    expected = (
        expected_path.read_text(encoding="utf-8").strip()
        if expected_path.is_file()
        else None
    )

    if gc_out != lm_out or gc_rc != lm_rc:
        record.update(
            {
                "status": "diverge",
                "gc_out": gc_out[-400:],
                "lm_out": lm_out[-400:],
            }
        )
        return record
    if gc_rc != 0:
        record["status"] = "parity-trap"
        return record
    if expected is not None:
        if gc_out.strip() != expected:
            record.update(
                {
                    "status": "diverge-from-expected",
                    "gc_out": gc_out[-400:],
                    "expected": expected[-400:],
                }
            )
            return record
    record["status"] = "parity"
    return record


def _tool_version(argv: list[str]) -> str:
    try:
        proc = _run(argv, timeout=10)
        return (proc.stdout or proc.stderr).strip().splitlines()[0]
    except Exception as exc:  # pragma: no cover - best-effort receipt field
        return f"unavailable: {exc}"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--filter-dir",
        action="append",
        default=[],
        help="fixture subdirectory prefix (repeatable); default is the "
        "curated RFC-011 semantic domain list",
    )
    parser.add_argument(
        "--fixture",
        action="append",
        default=[],
        help="explicit fixture path relative to tests/fixtures (repeatable)",
    )
    parser.add_argument("--workers", type=int, default=8)
    parser.add_argument(
        "--out",
        default=None,
        help="JSON receipt path (default .build/parity/report.json)",
    )
    args = parser.parse_args()

    compiler = os.environ.get("ARUKELLT_SELFHOST_WASM", "")
    if not compiler or not Path(compiler).is_file():
        print(
            "error: ARUKELLT_SELFHOST_WASM must point at the selfhost "
            "compiler runtime wasm",
            file=sys.stderr,
        )
        return 2

    selected: list[str] = list(args.fixture)
    if args.filter_dir:
        all_run = _manifest_run_fixtures()
        selected += [
            f
            for f in all_run
            if any(f.startswith(d) for d in args.filter_dir)
        ]
    if not selected:
        all_run = _manifest_run_fixtures()
        selected = [
            f for f in all_run if any(f.startswith(d) for d in DEFAULT_DOMAINS)
        ]
    selected = sorted(set(selected))

    out_dir = ROOT / OUT_DIR_REL
    out_dir.mkdir(parents=True, exist_ok=True)

    started = time.time()
    results: list[dict] = []
    with concurrent.futures.ThreadPoolExecutor(
        max_workers=max(1, args.workers)
    ) as pool:
        futures = {
            pool.submit(_probe_fixture, fixture, out_dir): fixture
            for fixture in selected
        }
        for future in concurrent.futures.as_completed(futures):
            record = future.result()
            results.append(record)
            status = record["status"]
            if status != "parity":
                print(f"  {status}: {record['fixture']}")

    counts: dict[str, int] = {}
    for record in results:
        counts[record["status"]] = counts.get(record["status"], 0) + 1

    receipt = {
        "generated_at": time.strftime("%Y-%m-%dT%H:%M:%S%z"),
        "compiler_wasm": str(compiler),
        "compiler_sha256": _sha256(Path(compiler)),
        "wasmtime": _tool_version(["wasmtime", "--version"]),
        "wasm_tools": _tool_version(["wasm-tools", "--version"]),
        "elapsed_seconds": round(time.time() - started, 1),
        "counts": counts,
        "results": sorted(results, key=lambda r: r["fixture"]),
    }
    out_path = Path(args.out) if args.out else out_dir / "report.json"
    out_path.write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")

    summary = " ".join(f"{k}={v}" for k, v in sorted(counts.items()))
    print(
        f"[cross-backend-parity] {len(results)} fixtures in "
        f"{receipt['elapsed_seconds']}s - {summary}"
    )
    print(f"[cross-backend-parity] receipt: {out_path}")
    bad = sum(
        v
        for k, v in counts.items()
        if k.startswith("diverge") or k.startswith("compile-fail")
    )
    return 1 if bad else 0


if __name__ == "__main__":
    raise SystemExit(main())
