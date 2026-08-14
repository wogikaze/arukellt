#!/usr/bin/env python3
"""Close gate for #674 — component dependency resolution, lock/cache, and external interop lanes."""
from __future__ import annotations
import importlib.util
import json
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
TOOL = ROOT / "scripts/component-deps.py"
WRAPPER = ROOT / "scripts/run/arukellt-selfhost.sh"
FIXTURE = ROOT / "tests/fixtures/component-deps"


def load_tool():
    spec = importlib.util.spec_from_file_location("component_deps", TOOL)
    assert spec and spec.loader
    mod = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = mod
    spec.loader.exec_module(mod)
    return mod


def expect_error(fn, needle: str) -> bool:
    try:
        fn()
    except ValueError as exc:
        return needle in str(exc)
    return False


def main() -> int:
    required = [
        TOOL,
        WRAPPER,
        ROOT / "docs/ark-toml.md",
        FIXTURE / "ark.toml",
        ROOT / "tests/component-interop/external/go/run.sh",
        ROOT / "tests/component-interop/external/c/run.sh",
        ROOT / "tests/component-interop/external/zig/run.sh",
        ROOT / "tests/component-interop/external/python-host/run.sh",
    ]
    for path in required:
        if not path.exists():
            print(f"gate-674: FAIL: missing {path.relative_to(ROOT)}", file=sys.stderr)
            return 1
    wrapper = WRAPPER.read_text(encoding="utf-8")
    if '"compose"' not in wrapper or '"--manifest"' not in wrapper or "scripts/component-deps.py" not in wrapper:
        print("gate-674: FAIL: arukellt compose does not route --manifest through component resolver", file=sys.stderr)
        return 1
    mod = load_tool()
    with tempfile.TemporaryDirectory(prefix="gate-674-") as td:
        root = Path(td)
        vendor = root / "vendor/greeter"
        vendor.mkdir(parents=True)
        component = vendor / "component.wasm"
        component.write_bytes(b"fixture-component-bytes")
        (vendor / "mod.wit").write_text("package test:greeter@0.1.0;\nworld provider { export greet: func(); }\n", encoding="utf-8")
        manifest = root / "ark.toml"
        manifest.write_text('[dependencies]\n"test:greeter" = { path = "vendor/greeter", package = "test:greeter", world = "provider" }\n', encoding="utf-8")
        deps = mod.resolve(manifest)
        if len(deps) != 1 or not deps[0].cached.is_file():
            print("gate-674: FAIL: dependency cache resolution failed", file=sys.stderr)
            return 1
        lock = json.loads((root / "ark.lock").read_text(encoding="utf-8"))
        if lock.get("version") != 1 or lock["components"][0]["name"] != "test:greeter":
            print("gate-674: FAIL: lockfile contract invalid", file=sys.stderr)
            return 1
        if len(lock["components"][0].get("sha256", "")) != 64:
            print("gate-674: FAIL: lockfile does not bind component content", file=sys.stderr)
            return 1
        manifest.write_text('[dependencies]\n"test:greeter" = { path = "vendor/greeter", package = "other:package", world = "provider" }\n', encoding="utf-8")
        if not expect_error(lambda: mod.resolve(manifest), "package mismatch"):
            print("gate-674: FAIL: package mismatch was not diagnosed", file=sys.stderr)
            return 1
        manifest.write_text('[dependencies]\n"test:greeter" = { path = "vendor/greeter", package = "test:greeter", world = "missing-world" }\n', encoding="utf-8")
        if not expect_error(lambda: mod.resolve(manifest), "incompatible world"):
            print("gate-674: FAIL: world mismatch was not diagnosed", file=sys.stderr)
            return 1
        manifest.write_text('[dependencies]\n"test:greeter" = { path = "vendor/greeter", package = "test:greeter", world = "provider" }\n', encoding="utf-8")
        component.unlink()
        if not expect_error(lambda: mod.resolve(manifest), "missing"):
            print("gate-674: FAIL: missing dependency was not diagnosed", file=sys.stderr)
            return 1
    source = TOOL.read_text(encoding="utf-8")
    for needle in ("package mismatch", "incompatible world", "wasm-tools", "ark.lock", ".build", "wac", "sha256"):
        if needle not in source:
            print(f"gate-674: FAIL: resolver missing {needle!r}", file=sys.stderr)
            return 1
    print("gate-674-component-composition-deps: PASS")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
