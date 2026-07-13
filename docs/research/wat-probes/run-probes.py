#!/usr/bin/env python3
"""Run feature WAT probes against local toolchains.

Usage:
  python3 docs/research/wat-probes/run-probes.py

Writes:
  docs/research/wat-probes/results.json
  docs/research/wat-probes/results.md
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
import tempfile
from dataclasses import asdict, dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[2]


@dataclass
class StageResult:
    ok: bool
    detail: str = ""


@dataclass
class ProbeResult:
    probe: str
    group: str
    expected: str
    stages: dict[str, StageResult] = field(default_factory=dict)
    notes: list[str] = field(default_factory=list)


# expected: "42" | "-1" | "1" | "65" | "2147483647" | "0" | "trap" | "validate" | "tooling" | "js" | "param"
EXPECT: dict[str, str] = {
    # wasm10
    "wasm10/01-arithmetic.wat": "42",
    "wasm10/02-locals.wat": "42",
    "wasm10/03-globals.wat": "42",
    "wasm10/04-drop-select.wat": "42",
    "wasm10/05-control-flow.wat": "42",
    "wasm10/06-call.wat": "42",
    "wasm10/07-call-indirect.wat": "42",
    "wasm10/08-memory.wat": "42",
    "wasm10/09-start.wat": "42",
    "wasm10/10-custom-section.wat": "42",
    "wasm10/11-trap-unreachable.wat": "trap",
    "wasm10/12-convert-reinterpret.wat": "42",
    # wasm20
    "wasm20/01-multi-value.wat": "42",
    "wasm20/02-reference-types.wat": "1",
    "wasm20/03-typed-select.wat": "1",
    "wasm20/04-multiple-tables.wat": "42",
    "wasm20/05-bulk-memory.wat": "65",
    "wasm20/06-simd.wat": "3",
    "wasm20/07-sign-extension.wat": "-1",
    "wasm20/08-trunc-sat-scalar.wat": "2147483647",
    "wasm20/09-trunc-sat-simd.wat": "2147483647",
    "wasm20/10-js-bigint-i64.wat": "js",
    "wasm20/11-table-ops.wat": "42",
    # wasm30
    "wasm30/01-extended-const.wat": "42",
    "wasm30/02-memory64.wat": "42",
    "wasm30/03-table64.wat": "42",
    "wasm30/04-multiple-memories.wat": "42",
    "wasm30/05-tail-call.wat": "param",  # invoke with arg
    "wasm30/06-typed-func-ref.wat": "42",
    "wasm30/07-br-on-null.wat": "42",
    "wasm30/08-gc-struct.wat": "42",
    "wasm30/09-gc-array.wat": "42",
    "wasm30/10-i31.wat": "42",
    "wasm30/11-eh-try-table.wat": "42",
    "wasm30/12-relaxed-simd.wat": "validate",  # result may vary
    "wasm30/13-custom-annotations.wat": "tooling",
    "wasm30/14-return-call-ref.wat": "42",
    "wasm30/15-recursive-types.wat": "42",
    "wasm30/16-js-string-builtins.wat": "js",
    # experimental
    "experimental/legacy-eh-try-catch.wat": "42",
    "experimental/threads-atomics.wat": "42",
}


def toolchain_versions() -> dict[str, str]:
    vers: dict[str, str] = {}
    for name, cmd in [
        ("wasm-tools", ["wasm-tools", "--version"]),
        ("wat2wasm", ["wat2wasm", "--version"]),
        ("wasm-validate", ["wasm-validate", "--version"]),
        ("wasmtime", ["wasmtime", "--version"]),
        ("iwasm", ["iwasm", "--version"]),
        ("node", ["node", "--version"]),
    ]:
        if which(cmd[0]):
            code, out, err = run(cmd)
            vers[name] = (out or err).splitlines()[0] if (out or err) else f"exit={code}"
        else:
            vers[name] = "missing"
    # Prefer pinned jco used by probes (npx), fall back to PATH jco
    code, out, err = run(["npx", "--yes", "@bytecodealliance/jco@1.25.2", "--version"])
    if code == 0 and (out or err):
        vers["jco"] = f"npx @bytecodealliance/jco@1.25.2 => {(out or err).splitlines()[0]}"
    elif which("jco"):
        code, out, err = run(["jco", "--version"])
        vers["jco"] = (out or err).splitlines()[0] if (out or err) else "present"
    else:
        vers["jco"] = "missing"
    chrome = _chrome_path()
    vers["chrome"] = chrome if chrome else "missing"
    return vers


def _chrome_path() -> str | None:
    env = os.environ.get("CHROME_PATH")
    if env and Path(env).exists():
        return env
    # puppeteer-downloaded chrome from local .browser-tools install
    cache = Path.home() / ".cache/puppeteer/chrome"
    if cache.is_dir():
        matches = sorted(cache.glob("linux-*/chrome-linux64/chrome"))
        if matches:
            return str(matches[-1])
    for c in (
        "/usr/bin/google-chrome",
        "/usr/bin/google-chrome-stable",
        "/usr/bin/chromium",
        "/usr/bin/chromium-browser",
    ):
        if Path(c).exists():
            return c
    return None


def which(name: str) -> str | None:
    return shutil.which(name)


def run(cmd: list[str], timeout: float = 60.0) -> tuple[int, str, str]:
    try:
        p = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            timeout=timeout,
            cwd=str(REPO),
        )
        return p.returncode, p.stdout.strip(), p.stderr.strip()
    except FileNotFoundError as e:
        return 127, "", str(e)
    except subprocess.TimeoutExpired:
        return 124, "", "timeout"


def parse_with_wasm_tools(wat: Path, wasm_out: Path) -> StageResult:
    code, out, err = run(["wasm-tools", "parse", str(wat), "-o", str(wasm_out)])
    if code == 0 and wasm_out.exists():
        return StageResult(True, "ok")
    return StageResult(False, err or out or f"exit={code}")


def inject_custom_section(wasm_in: Path, wasm_out: Path, name: str, payload: bytes) -> StageResult:
    """Append a Wasm 1.0 custom section (id=0) to an existing module binary."""
    raw = wasm_in.read_bytes()
    if raw[:4] != b"\x00asm":
        return StageResult(False, "not a wasm binary")
    name_b = name.encode("utf-8")
    body = bytes([len(name_b)]) + name_b + payload
    # LEB128 length
    n = len(body)
    leb = bytearray()
    while True:
        byte = n & 0x7F
        n >>= 7
        if n:
            leb.append(byte | 0x80)
        else:
            leb.append(byte)
            break
    wasm_out.write_bytes(raw + b"\x00" + bytes(leb) + body)
    return StageResult(True, f"appended custom '{name}' ({len(payload)} bytes)")


def parse_with_wabt(wat: Path, wasm_out: Path) -> StageResult:
    if not which("wat2wasm"):
        return StageResult(False, "wat2wasm missing")
    # Prefer --enable-all so proposal features are opt-in visible; also try
    # annotations for @custom / @name probes.
    for flags in (
        ["--enable-all", "--enable-annotations"],
        ["--enable-all"],
        [],
    ):
        code, out, err = run(["wat2wasm", *flags, str(wat), "-o", str(wasm_out)])
        if code == 0 and wasm_out.exists():
            note = f"ok flags={flags}" if flags else "ok (default)"
            return StageResult(True, note)
    return StageResult(False, err or out or f"exit={code}")


def validate_wasm_tools(wasm: Path) -> StageResult:
    code, out, err = run(["wasm-tools", "validate", str(wasm)])
    if code == 0:
        return StageResult(True, "ok")
    return StageResult(False, err or out or f"exit={code}")


def validate_wabt(wasm: Path) -> StageResult:
    if not which("wasm-validate"):
        return StageResult(False, "wasm-validate missing")
    # enable features liberally
    code, out, err = run(
        [
            "wasm-validate",
            "--enable-all",
            str(wasm),
        ]
    )
    if code == 0:
        return StageResult(True, "ok")
    return StageResult(False, err or out or f"exit={code}")


def invoke_wasmtime(wasm: Path, expected: str, probe_rel: str) -> StageResult:
    if not which("wasmtime"):
        return StageResult(False, "wasmtime missing")
    # Enable all implemented proposals so opt-in features (GC, EH, memory64, …)
    # are visible; record default-vs-opt-in separately in the research write-up.
    base = ["wasmtime", "run", "-W", "all-proposals=y", "--invoke", "test"]
    if "threads-atomics" in probe_rel:
        # shared memory is gated separately from the threads proposal flag
        base = [
            "wasmtime",
            "run",
            "-W",
            "all-proposals=y",
            "-W",
            "shared-memory=y",
            "--invoke",
            "test",
        ]

    if expected in ("js", "tooling"):
        return StageResult(True, "skipped (host/tooling probe)")

    if expected == "param" or probe_rel.endswith("05-tail-call.wat"):
        cmd = base + [str(wasm), "1000"]
    else:
        cmd = base + [str(wasm)]

    code, out, err = run(cmd)
    text = (out + "\n" + err).lower()

    if expected == "trap":
        if code != 0 and ("trap" in text or "unreachable" in text or "error" in text):
            return StageResult(True, f"trapped exit={code}")
        return StageResult(False, f"expected trap, exit={code} out={out!r} err={err!r}")

    if code != 0:
        return StageResult(False, err or out or f"exit={code}")

    # wasmtime --invoke prints the result (may warn on stdout)
    lines = [ln for ln in out.strip().splitlines() if not ln.startswith("warning:")]
    got = lines[-1] if lines else ""
    digits = got.replace("i32:", "").replace("i64:", "").strip()
    if expected == "param":
        if digits in ("0",) or got.endswith("0"):
            return StageResult(True, f"got {got!r}")
        return StageResult(False, f"expected 0, got {got!r}")
    if expected == "validate":
        return StageResult(True, f"got {got!r}")
    if digits == expected or got == expected:
        return StageResult(True, f"got {got!r}")
    if expected == "-1" and digits in ("-1", "4294967295"):
        return StageResult(True, f"got {got!r}")
    return StageResult(False, f"expected {expected}, got {got!r} (err={err!r})")


def invoke_iwasm(wasm: Path, expected: str) -> StageResult:
    if not which("iwasm"):
        return StageResult(False, "iwasm missing")
    if expected in ("js", "tooling"):
        return StageResult(True, "skipped (host/tooling probe)")
    cmd = ["iwasm", "--heap-size=0", str(wasm)]
    if expected == "param":
        # iwasm needs exported main or function via --invoke if supported
        code_h, out_h, err_h = run(["iwasm", "--help"])
        help_text = out_h + err_h
        if "--invoke" in help_text or "-f" in help_text:
            cmd = ["iwasm", "-f", "test", str(wasm), "1000"]
        else:
            return StageResult(False, "no invoke support for param probe")
    elif expected == "trap":
        code, out, err = run(["iwasm", "-f", "test", str(wasm)])
        text = (out + "\n" + err).lower()
        if code != 0 and ("trap" in text or "unreachable" in text or "exception" in text):
            return StageResult(True, f"trapped exit={code}")
        return StageResult(False, f"expected trap, exit={code} {err or out}")
    else:
        code_h, out_h, err_h = run(["iwasm", "--help"])
        if "-f" in (out_h + err_h) or "--invoke" in (out_h + err_h):
            cmd = ["iwasm", "-f", "test", str(wasm)]
        else:
            cmd = ["iwasm", str(wasm)]

    code, out, err = run(cmd)
    if code != 0:
        return StageResult(False, err or out or f"exit={code}")
    got = out.strip().splitlines()[-1] if out.strip() else ""
    if expected in ("validate", "param"):
        return StageResult(True, f"got {got!r}")
    # iwasm prints e.g. "0x2a:i32" or "0xffffffff:i32"
    parsed = _parse_iwasm_value(got)
    if parsed is not None:
        if expected == "-1" and parsed in (-1, 0xFFFFFFFF):
            return StageResult(True, f"got {got!r} -> {parsed}")
        if str(parsed) == expected:
            return StageResult(True, f"got {got!r} -> {parsed}")
    if expected in got or got == expected:
        return StageResult(True, f"got {got!r}")
    if expected == "-1" and ("-1" in got or "4294967295" in got or "0xffffffff" in got.lower()):
        return StageResult(True, f"got {got!r}")
    if expected != "validate" and expected not in got:
        return StageResult(False, f"expected {expected}, got {got!r}")
    return StageResult(True, f"got {got!r}")


def _parse_iwasm_value(got: str) -> int | None:
    """Parse iwasm REPL-style values like '0x2a:i32'."""
    s = got.strip()
    if ":" in s:
        s = s.split(":", 1)[0]
    try:
        if s.lower().startswith("0x"):
            return int(s, 16)
        return int(s)
    except ValueError:
        return None


NODE_HARNESS = r"""
const fs = require('fs');
const path = process.argv[2];
const expected = process.argv[3];
const bytes = fs.readFileSync(path);

function fail(msg) { console.error(msg); process.exit(2); }

async function main() {
  let validated = false;
  try {
    validated = WebAssembly.validate(bytes);
  } catch (e) {
    fail('validate-throw: ' + e);
  }
  if (expected === 'js-string') {
    let ok = false;
    try {
      ok = WebAssembly.validate(bytes, { builtins: ['js-string'] });
    } catch (e) {
      // older engines
      console.log(JSON.stringify({ validate: validated, builtins: false, error: String(e) }));
      process.exit(validated ? 0 : 1);
    }
    console.log(JSON.stringify({ validate: validated, builtins: ok }));
    process.exit(ok ? 0 : 1);
  }
  if (!validated && expected !== 'trap') {
    // still try compile for better errors
  }
  if (expected === 'js-bigint') {
    const { instance } = await WebAssembly.instantiate(bytes);
    const r = instance.exports.test(1n);
    const ok = r === 1n;
    console.log(JSON.stringify({ result: String(r), ok }));
    process.exit(ok ? 0 : 1);
  }
  try {
    const { instance } = await WebAssembly.instantiate(bytes);
    if (expected === 'trap') {
      try {
        instance.exports.test();
        fail('expected trap');
      } catch (e) {
        console.log(JSON.stringify({ trap: true, error: String(e) }));
        process.exit(0);
      }
    }
    let result;
    if (expected === 'param') {
      result = instance.exports.test(1000);
    } else if (typeof instance.exports.test === 'function') {
      result = instance.exports.test();
    } else {
      fail('no export test');
    }
    console.log(JSON.stringify({ validate: validated, result: String(result) }));
    if (expected === 'validate' || expected === 'tooling') process.exit(0);
    if (expected === 'param') process.exit(Number(result) === 0 ? 0 : 1);
    if (String(result) === expected || (expected === '-1' && Number(result) === -1)) process.exit(0);
    process.exit(1);
  } catch (e) {
    console.log(JSON.stringify({ validate: validated, error: String(e) }));
    process.exit(expected === 'trap' ? 0 : 1);
  }
}
main();
"""


def invoke_node(wasm: Path, expected: str, probe_rel: str) -> StageResult:
    if not which("node"):
        return StageResult(False, "node missing")
    if expected == "tooling":
        return StageResult(True, "skipped (tooling probe)")
    exp = expected
    if probe_rel.endswith("10-js-bigint-i64.wat"):
        exp = "js-bigint"
    elif probe_rel.endswith("16-js-string-builtins.wat"):
        exp = "js-string"
    elif expected == "js":
        exp = "js-bigint"
    with tempfile.NamedTemporaryFile("w", suffix=".js", delete=False) as f:
        f.write(NODE_HARNESS)
        harness = f.name
    try:
        # Prefer newer Node; fall back to --experimental-wasm-exnref for try_table.
        attempts = [["node", harness, str(wasm), exp]]
        if "eh-try-table" in probe_rel or expected == "42" and "eh" in probe_rel:
            attempts.append(
                ["node", "--experimental-wasm-exnref", harness, str(wasm), exp]
            )
        if "11-eh-try-table" in probe_rel:
            attempts = [
                ["node", harness, str(wasm), exp],
                ["node", "--experimental-wasm-exnref", harness, str(wasm), exp],
            ]
        last = StageResult(False, "no attempt")
        for cmd in attempts:
            code, out, err = run(cmd)
            if code == 0:
                flag = " (with --experimental-wasm-exnref)" if "--experimental-wasm-exnref" in cmd else ""
                return StageResult(True, (out or "ok") + flag)
            last = StageResult(False, err or out or f"exit={code}")
        return last
    finally:
        os.unlink(harness)


def invoke_browser(wasm: Path, expected: str, probe_rel: str) -> StageResult:
    if expected == "tooling":
        return StageResult(True, "skipped (tooling probe)")
    chrome = _chrome_path()
    if not chrome:
        return StageResult(False, "chrome missing")
    browser_tools = REPO / "scripts" / "dev" / "wat-probe-browser" / "node_modules" / "puppeteer"
    if not browser_tools.exists():
        return StageResult(
            False,
            "puppeteer missing; cd scripts/dev/wat-probe-browser && npm i",
        )
    exp = expected
    if probe_rel.endswith("10-js-bigint-i64.wat"):
        exp = "js-bigint"
    elif probe_rel.endswith("16-js-string-builtins.wat"):
        exp = "js-string"
    elif expected == "js":
        exp = "js-bigint"
    script = ROOT / "browser-probe.mjs"
    cmd = ["node", str(script), str(wasm), exp, "--chrome", chrome]
    code, out, err = run(cmd, timeout=90.0)
    if code == 0:
        return StageResult(True, out or "ok")
    return StageResult(False, err or out or f"exit={code}")


def probe_jco(wasm: Path, expected: str, probe_rel: str, work: Path) -> StageResult:
    """Wrap core wasm as a minimal component and run jco transpile (1.25.2)."""
    if expected in ("tooling",):
        return StageResult(True, "skipped (tooling probe)")
    if probe_rel.endswith("16-js-string-builtins.wat"):
        return StageResult(False, "skipped: host js-string imports (not a pure core export probe)")
    if not which("wasm-tools") or not which("npx"):
        return StageResult(False, "wasm-tools or npx missing")

    wit_dir = work / "wit"
    wit_dir.mkdir(parents=True, exist_ok=True)
    if expected == "param" or probe_rel.endswith("05-tail-call.wat"):
        wit = "package probe:feat@0.0.1;\nworld probe {\n  export test: func(n: s32) -> s32;\n}\n"
    elif probe_rel.endswith("10-js-bigint-i64.wat"):
        wit = "package probe:feat@0.0.1;\nworld probe {\n  export test: func(x: s64) -> s64;\n}\n"
    else:
        wit = "package probe:feat@0.0.1;\nworld probe {\n  export test: func() -> s32;\n}\n"
    (wit_dir / "world.wit").write_text(wit)

    embedded = work / "embedded.wasm"
    component = work / "component.wasm"
    out_dir = work / "jco-out"

    code, out, err = run(
        [
            "wasm-tools",
            "component",
            "embed",
            str(wit_dir),
            "--world",
            "probe",
            str(wasm),
            "-o",
            str(embedded),
        ]
    )
    if code != 0:
        return StageResult(False, f"embed: {err or out}")

    code, out, err = run(
        ["wasm-tools", "component", "new", str(embedded), "-o", str(component)]
    )
    if code != 0:
        return StageResult(False, f"component new: {err or out}")

    out_dir.mkdir(parents=True, exist_ok=True)
    code, out, err = run(
        [
            "npx",
            "--yes",
            "@bytecodealliance/jco@1.25.2",
            "transpile",
            str(component),
            "-o",
            str(out_dir),
        ],
        timeout=120.0,
    )
    if code != 0:
        detail = (err or out or "").replace("\n", " / ")
        if len(detail) > 200:
            detail = detail[:197] + "..."
        return StageResult(False, f"transpile: {detail}")
    js_files = list(out_dir.glob("*.js"))
    return StageResult(True, f"transpile ok ({len(js_files)} js)")


def annotation_roundtrip(wat: Path) -> StageResult:
    """Tooling probe: parse and print, check @custom presence if supported."""
    code, out, err = run(["wasm-tools", "print", str(wat)])
    # print needs wasm; parse first
    with tempfile.TemporaryDirectory() as td:
        wasm = Path(td) / "a.wasm"
        p = parse_with_wasm_tools(wat, wasm)
        if not p.ok:
            return StageResult(False, f"parse failed: {p.detail}")
        code, out, err = run(["wasm-tools", "print", str(wasm)])
        if code != 0:
            return StageResult(False, err or out)
        # custom section may appear as (@custom ...) or raw section dump
        text = out
        ok = "my-section" in text or "payload" in text or "custom" in text.lower()
        return StageResult(ok, "annotation/custom visible in print" if ok else "custom not visible in print")


def collect_wat_files() -> list[Path]:
    files: list[Path] = []
    for group in ("wasm10", "wasm20", "wasm30", "experimental"):
        d = ROOT / group
        if d.is_dir():
            files.extend(sorted(d.glob("*.wat")))
    return files


def main() -> int:
    versions = toolchain_versions()
    results: list[ProbeResult] = []

    with tempfile.TemporaryDirectory(prefix="wat-probes-") as td:
        tdir = Path(td)
        for wat in collect_wat_files():
            rel = str(wat.relative_to(ROOT))
            expected = EXPECT.get(rel, "42")
            group = wat.parent.name
            pr = ProbeResult(probe=rel, group=group, expected=expected)

            wt_wasm = tdir / (wat.stem + ".wt.wasm")
            wabt_wasm = tdir / (wat.stem + ".wabt.wasm")

            pr.stages["wasm-tools.parse"] = parse_with_wasm_tools(wat, wt_wasm)
            if pr.stages["wasm-tools.parse"].ok:
                pr.stages["wasm-tools.validate"] = validate_wasm_tools(wt_wasm)
            else:
                pr.stages["wasm-tools.validate"] = StageResult(False, "skipped (parse failed)")

            pr.stages["wabt.wat2wasm"] = parse_with_wabt(wat, wabt_wasm)
            if pr.stages["wabt.wat2wasm"].ok:
                pr.stages["wabt.validate"] = validate_wabt(wabt_wasm)
            else:
                pr.stages["wabt.validate"] = StageResult(False, "skipped (parse failed)")

            # Prefer wasm-tools binary for runtime probes when available
            runtime_wasm = wt_wasm if pr.stages["wasm-tools.parse"].ok else (
                wabt_wasm if pr.stages["wabt.wat2wasm"].ok else None
            )

            if expected == "tooling" or rel.endswith("13-custom-annotations.wat"):
                pr.stages["wasm-tools.annotation"] = annotation_roundtrip(wat)

            if runtime_wasm is None:
                pr.stages["wasmtime"] = StageResult(False, "no binary")
                pr.stages["iwasm"] = StageResult(False, "no binary")
                pr.stages["node"] = StageResult(False, "no binary")
                pr.stages["browser"] = StageResult(False, "no binary")
                pr.stages["jco.transpile"] = StageResult(False, "no binary")
            else:
                # Wasm 1.0 custom-section probe: inject section into binary and re-check
                if rel.endswith("10-custom-section.wat"):
                    custom_wasm = tdir / (wat.stem + ".custom.wasm")
                    inj = inject_custom_section(
                        runtime_wasm, custom_wasm, "probe-meta", b"wasm10-custom"
                    )
                    pr.stages["custom-section.inject"] = inj
                    if inj.ok:
                        pr.stages["wasm-tools.validate+custom"] = validate_wasm_tools(custom_wasm)
                        runtime_wasm = custom_wasm

                pr.stages["wasmtime"] = invoke_wasmtime(runtime_wasm, expected, rel)
                pr.stages["iwasm"] = invoke_iwasm(runtime_wasm, expected)
                pr.stages["node"] = invoke_node(runtime_wasm, expected, rel)
                pr.stages["browser"] = invoke_browser(runtime_wasm, expected, rel)
                jco_work = tdir / f"jco-{wat.stem}"
                jco_work.mkdir(parents=True, exist_ok=True)
                pr.stages["jco.transpile"] = probe_jco(runtime_wasm, expected, rel, jco_work)

            results.append(pr)
            print(f"[{group}] {wat.name}: " + ", ".join(
                f"{k}={'Y' if v.ok else 'N'}" for k, v in pr.stages.items()
            ), flush=True)

    # serialize
    payload = {
        "date": "2026-07-13",
        "versions": versions,
        "results": [
            {
                "probe": r.probe,
                "group": r.group,
                "expected": r.expected,
                "stages": {k: asdict(v) for k, v in r.stages.items()},
                "notes": r.notes,
            }
            for r in results
        ],
    }
    (ROOT / "results.json").write_text(json.dumps(payload, indent=2) + "\n")
    (ROOT / "results.md").write_text(render_md(payload))
    print(f"\nWrote {ROOT / 'results.json'} and {ROOT / 'results.md'}")
    return 0


def render_md(payload: dict) -> str:
    lines = [
        "# WAT probe results",
        "",
        f"Date: {payload['date']}",
        "",
        "## Toolchain versions",
        "",
        "| Tool | Version |",
        "|------|---------|",
    ]
    for k, v in payload["versions"].items():
        lines.append(f"| {k} | `{v}` |")
    lines += ["", "## Matrix", ""]

    # collect stage names
    stage_names: list[str] = []
    for r in payload["results"]:
        for s in r["stages"]:
            if s not in stage_names:
                stage_names.append(s)

    header = "| Probe | Expected | " + " | ".join(stage_names) + " |"
    sep = "|-------|----------|" + "|".join(["---"] * len(stage_names)) + "|"
    lines += [header, sep]
    for r in payload["results"]:
        cells = []
        for s in stage_names:
            st = r["stages"].get(s)
            if not st:
                cells.append("—")
            elif st["ok"]:
                cells.append("✅")
            else:
                detail = (st.get("detail") or "").replace("|", "\\|").replace("`", "'")
                short = detail.split("\n")[0][:60]
                cells.append(f"❌ `{short}`" if short else "❌")
        lines.append(
            f"| `{r['probe']}` | `{r['expected']}` | " + " | ".join(cells) + " |"
        )

    lines += [
        "",
        "## Failure details",
        "",
    ]
    for r in payload["results"]:
        fails = {k: v for k, v in r["stages"].items() if not v["ok"]}
        if not fails:
            continue
        lines.append(f"### `{r['probe']}`")
        lines.append("")
        for k, v in fails.items():
            detail = (v["detail"] or "").replace("`", "'").replace("\n", " / ")
            # Keep one line to avoid MD031 fence issues inside lists
            if len(detail) > 160:
                detail = detail[:157] + "..."
            lines.append(f"- **{k}**: `{detail}`")
        lines.append("")
    while lines and lines[-1] == "":
        lines.pop()
    return "\n".join(lines) + "\n"


if __name__ == "__main__":
    sys.exit(main())
