#!/usr/bin/env python3
"""Init template gate (issue #464).

Exercises ``arukellt init --list-templates`` and ``--template`` scaffolding.
"""

from __future__ import annotations

import shutil
import subprocess
import sys
from pathlib import Path


def _repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def _compiler(root: Path) -> Path | None:
    sys.path.insert(0, str(root))
    from scripts.selfhost.checks import resolve_ide_gate_compiler_wasm

    return resolve_ide_gate_compiler_wasm(root)


def _run(
    compiler: Path,
    root: Path,
    args: list[str],
    *,
    cwd: Path | None = None,
    extra_dirs: list[Path] | None = None,
) -> subprocess.CompletedProcess[str]:
    wasmtime = shutil.which("wasmtime")
    assert wasmtime is not None
    work = cwd or root
    cmd = [wasmtime, "run", "--dir", str(root)]
    for d in extra_dirs or []:
        cmd.extend(["--dir", str(d)])
    cmd.extend([str(compiler), "--", *args])
    return subprocess.run(
        cmd,
        cwd=str(work),
        capture_output=True,
        text=True,
        timeout=120,
    )


def main() -> int:
    root = _repo_root()
    compiler = _compiler(root)
    if compiler is None:
        print("error: no selfhost compiler wasm", file=sys.stderr)
        return 1

    failures = 0

    r = _run(compiler, root, ["init", "--list-templates"])
    if r.returncode != 0:
        print(f"FAIL init --list-templates exit {r.returncode}", file=sys.stderr)
        failures += 1
    else:
        out = r.stdout + r.stderr
        for name in ("minimal", "cli", "with-tests", "wasi-host"):
            if name not in out:
                print(f"FAIL list-templates missing {name}", file=sys.stderr)
                failures += 1

    scratch = root / ".build" / "init-template-tests"
    scratch.mkdir(parents=True, exist_ok=True)

    for template in ("minimal", "cli", "with-tests", "wasi-host"):
        proj = scratch / f"proj-{template}"
        if proj.exists():
            shutil.rmtree(proj)
        proj.mkdir(parents=True, exist_ok=True)
        rel = str(proj.relative_to(root))
        r = _run(compiler, root, ["init", rel, "--template", template])
        if r.returncode != 0:
            print(f"FAIL init --template {template}: exit {r.returncode}", file=sys.stderr)
            print((r.stdout + r.stderr)[:400], file=sys.stderr)
            failures += 1
            continue
        main_ark = proj / "main.ark"
        ark_toml = proj / "ark.toml"
        if not main_ark.is_file() or not ark_toml.is_file():
            print(f"FAIL init --template {template}: missing scaffold files", file=sys.stderr)
            failures += 1
            continue
        check_path = f"{rel}/main.ark"
        check_args = ["check", check_path]
        if template == "wasi-host":
            check_args.extend(["--target", "wasm32-gc"])
        r_check = _run(compiler, root, check_args)
        if r_check.returncode != 0:
            print(f"FAIL check after {template}: {(r_check.stdout + r_check.stderr)[:300]!r}",
                  file=sys.stderr)
            failures += 1
            continue
        if template == "with-tests":
            r_test = _run(compiler, root, ["test", check_path])
            if r_test.returncode != 0:
                print(f"FAIL test after with-tests: {(r_test.stdout + r_test.stderr)[:300]!r}",
                      file=sys.stderr)
                failures += 1
                continue
        shutil.rmtree(proj)

    if failures:
        print(f"init-templates: {failures} failure(s)", file=sys.stderr)
        return 1
    print("init-templates: pass")
    return 0


if __name__ == "__main__":
    sys.exit(main())
