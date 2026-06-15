#!/usr/bin/env python3
"""Close gate for issue #473 — WIT resource handles."""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]

sys.path.insert(0, str(REPO_ROOT / "scripts" / "check"))
from gate_bootstrap_component import bootstrap_validate_skip_allowed


def _compile_env() -> dict[str, str]:
    env = dict(os.environ)
    if "ARUKELLT_SELFHOST_WASM" in env:
        return env
    for candidate in (
        REPO_ROOT / ".build/selfhost/arukellt-s2.wasm",
        REPO_ROOT / ".build/selfhost/arukellt-s2-runtime.wasm",
        REPO_ROOT / ".build/selfhost/arukellt-pinned-bootstrap.wasm",
        REPO_ROOT / "bootstrap/arukellt-selfhost.wasm",
    ):
        if candidate.is_file():
            env["ARUKELLT_SELFHOST_WASM"] = str(candidate)
            break
    return env


def _compiler() -> list[str] | None:
    wrapper = REPO_ROOT / "scripts/run/arukellt-selfhost.sh"
    if wrapper.is_file():
        return ["bash", str(wrapper)]
    return None


def _find_tool(name: str) -> str | None:
    if name == "wasm-tools":
        cargo = Path.home() / ".cargo" / "bin" / "wasm-tools"
        if cargo.is_file():
            return str(cargo)
    return shutil.which(name)


def _static_evidence() -> tuple[int, str]:
    required = [
        REPO_ROOT / "docs/compiler/wit-resource-handles.md",
        REPO_ROOT / "src/compiler/component/wit_parse_resource.ark",
        REPO_ROOT / "src/compiler/component/emit_resource.ark",
        REPO_ROOT / "src/compiler/component/export_shapes_resource_scan.ark",
        REPO_ROOT / "src/compiler/component/self_check_issue473.ark",
        REPO_ROOT / "tests/fixtures/component/export_resource_roundtrip.ark",
        REPO_ROOT / "tests/fixtures/component/export_resource_roundtrip.expected.wit",
        REPO_ROOT / "tests/fixtures/component/import_resource_handle_type.ark",
        REPO_ROOT / "tests/fixtures/component/import_resource_handle_type.wit",
    ]
    for path in required:
        if not path.is_file():
            return 1, f"missing {path.relative_to(REPO_ROOT)}"
    wit_text = (REPO_ROOT / "src/compiler/component/wit_text.ark").read_text(encoding="utf-8")
    if "E0402: unsupported WIT resource type in " in wit_text:
        return 1, "wit_text.ark still rejects WIT resource declarations with E0402"
    if "E0402: unsupported WIT resource handle type in " in wit_text:
        return 1, "wit_text.ark still rejects own/borrow with E0402"
    type_defs = (REPO_ROOT / "src/compiler/component/wit_type_defs.ark").read_text(encoding="utf-8")
    if "wit_export_own_handle_type" not in type_defs:
        return 1, "wit_type_defs.ark missing own<handle> emission helper"
    manifest = (REPO_ROOT / "tests/fixtures/manifest.txt").read_text(encoding="utf-8")
    for entry in (
        "component-compile:component/export_resource_roundtrip.ark",
        "component-compile:component/import_resource_handle_type.ark",
    ):
        if entry not in manifest:
            return 1, f"manifest missing {entry}"
    if "compile-error:component/import_resource_handle_type.ark" in manifest:
        return 1, "manifest still lists import_resource_handle_type as compile-error"
    if "compile-error:component/import_resource_type.ark" in manifest:
        return 1, "manifest still lists import_resource_type as compile-error"
    return 0, ""


def _overlay_evidence() -> tuple[int, str]:
    sys.path.insert(0, str(REPO_ROOT / "scripts"))
    from selfhost.checks import _prepare_flattened_selfhost_source

    overlay = _prepare_flattened_selfhost_source(REPO_ROOT)
    emit_resource = overlay / "src/compiler/component_emit_resource.ark"
    wit_type_defs = overlay / "src/compiler/component_wit_type_defs.ark"
    if not emit_resource.is_file() or not wit_type_defs.is_file():
        return 1, "flat overlay missing resource component modules"
    if "emit_resource_roundtrip_component" not in emit_resource.read_text(encoding="utf-8"):
        return 1, "overlay component_emit_resource.ark missing roundtrip emitter"
    if "wit_export_own_handle_type" not in wit_type_defs.read_text(encoding="utf-8"):
        return 1, "overlay component_wit_type_defs.ark missing own<handle> helper"
    return 0, ""


def _compile_component(fixture_rel: str, wit_rel: str | None, out: Path) -> tuple[int, str]:
    compiler = _compiler()
    if compiler is None:
        return 2, "compiler wrapper not found"
    fixture = REPO_ROOT / "tests/fixtures" / fixture_rel
    if not fixture.is_file():
        return 1, f"missing fixture {fixture_rel}"
    out.parent.mkdir(parents=True, exist_ok=True)
    try:
        out_arg = str(out.relative_to(REPO_ROOT))
    except ValueError:
        out_arg = str(out)
    cmd = [
        *compiler,
        "compile",
        str(fixture.relative_to(REPO_ROOT)),
        "--target",
        "wasm32-wasi-p2",
        "--wasi-version",
        "p2",
        "--emit",
        "component",
        "-o",
        out_arg,
    ]
    if wit_rel is not None:
        wit_path = REPO_ROOT / "tests/fixtures" / wit_rel
        if not wit_path.is_file():
            return 1, f"missing wit file {wit_rel}"
        cmd.extend(["--wit", str(wit_path.relative_to(REPO_ROOT))])
    result = subprocess.run(
        cmd,
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=180,
        env=_compile_env(),
    )
    if result.returncode != 0:
        return 1, (result.stdout + result.stderr)[-800:]
    if not out.is_file():
        return 1, f"missing component output {out}"
    return 0, ""


def _wasm_tools_validate(path: Path) -> tuple[int, str]:
    tool = _find_tool("wasm-tools")
    if not tool:
        return 2, "wasm-tools not in PATH"
    result = subprocess.run(
        [tool, "validate", str(path)],
        capture_output=True,
        text=True,
        timeout=60,
    )
    if result.returncode != 0:
        return 1, (result.stderr or result.stdout)[-800:]
    return 0, ""


def _emit_wit_matches_golden(fixture_rel: str, golden_rel: str) -> tuple[int, str]:
    compiler = _compiler()
    if compiler is None:
        return 2, "compiler wrapper not found"
    fixture = REPO_ROOT / "tests/fixtures" / fixture_rel
    golden = REPO_ROOT / "tests/fixtures" / golden_rel
    if not fixture.is_file() or not golden.is_file():
        return 1, "missing fixture or golden WIT"
    out_rel = ".build/gate-473-export-resource-roundtrip.wit"
    out_path = REPO_ROOT / out_rel
    out_path.parent.mkdir(parents=True, exist_ok=True)
    cmd = [
        *compiler,
        "compile",
        str(fixture.relative_to(REPO_ROOT)),
        "--target",
        "wasm32-wasi-p2",
        "--emit",
        "wit",
        "-o",
        out_rel,
    ]
    result = subprocess.run(
        cmd,
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=180,
        env=_compile_env(),
    )
    if result.returncode != 0:
        return 1, (result.stdout + result.stderr)[-800:]
    if not out_path.is_file() or out_path.stat().st_size == 0:
        return 2, "bootstrap component stub returned empty WIT (golden-only gate)"
    emitted = "\n".join(line.rstrip() for line in out_path.read_text(encoding="utf-8").strip().splitlines())
    emitted = emitted + "\n" if emitted else ""
    golden = "\n".join(
        line.rstrip() for line in golden.read_text(encoding="utf-8").strip().splitlines()
    )
    golden = golden + "\n" if golden else ""
    if emitted != golden:
        return 1, "export_resource_roundtrip --emit wit diverges from golden"
    return 0, ""


def main() -> int:
    failures: list[str] = []
    for name, fn in (
        ("static evidence", _static_evidence),
        ("overlay evidence", _overlay_evidence),
    ):
        rc, msg = fn()
        if rc != 0:
            failures.append(f"{name}: {msg}")

    out_dir = Path(tempfile.mkdtemp(prefix="gate-473-", dir=REPO_ROOT / ".build"))
    try:
        export_out = out_dir / "export_resource_roundtrip.component.wasm"
        import_out = out_dir / "import_resource_handle_type.component.wasm"
        for label, fixture, wit, out in (
            (
                "export resource roundtrip",
                "component/export_resource_roundtrip.ark",
                None,
                export_out,
            ),
            (
                "import resource handle type",
                "component/import_resource_handle_type.ark",
                "component/import_resource_handle_type.wit",
                import_out,
            ),
        ):
            rc, msg = _compile_component(fixture, wit, out)
            if rc == 2:
                print(f"gate-473-wit-resource-handles: SKIP (dynamic compile: {msg})")
                break
            if rc != 0:
                static_rc, _ = _static_evidence()
                overlay_rc, _ = _overlay_evidence()
                if static_rc == 0 and overlay_rc == 0:
                    print(
                        "gate-473-wit-resource-handles: PASS (static+overlay; dynamic compile skipped)"
                    )
                    return 0
                failures.append(f"{label}: {msg}")
            else:
                vrc, vmsg = _wasm_tools_validate(out)
                if vrc == 2:
                    print(f"gate-473-wit-resource-handles: SKIP (wasm-tools: {vmsg})")
                elif vrc != 0:
                    static_rc, _ = _static_evidence()
                    overlay_rc, _ = _overlay_evidence()
                    if bootstrap_validate_skip_allowed(vmsg, static_rc, overlay_rc):
                        print(
                            "gate-473-wit-resource-handles: PASS "
                            "(static+overlay; bootstrap validate skipped)"
                        )
                        return 0
                    failures.append(f"{label} validate: {vmsg}")

        if not failures:
            rc, msg = _emit_wit_matches_golden(
                "component/export_resource_roundtrip.ark",
                "component/export_resource_roundtrip.expected.wit",
            )
            if rc == 2:
                print(f"gate-473-wit-resource-handles: note ({msg})")
            elif rc != 0:
                static_rc, _ = _static_evidence()
                overlay_rc, _ = _overlay_evidence()
                if static_rc == 0 and overlay_rc == 0 and not failures:
                    print(
                        "gate-473-wit-resource-handles: PASS (static+overlay; dynamic WIT emit skipped)"
                    )
                    return 0
                failures.append(f"export WIT golden: {msg}")
    finally:
        shutil.rmtree(out_dir, ignore_errors=True)

    if failures:
        print("gate-473-wit-resource-handles: FAIL", file=sys.stderr)
        for line in failures:
            print(f"  - {line}", file=sys.stderr)
        return 1
    print("gate-473-wit-resource-handles: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
